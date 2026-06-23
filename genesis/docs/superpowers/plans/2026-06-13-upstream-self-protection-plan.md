# Upstream Self-Protection — Implementation Plan

> For agentic workers: REQUIRED SUB-SKILL: superpowers:subagent-driven-development (or executing-plans). Steps use checkbox (- [ ]) syntax.

**Goal:** Make doorway warm-up survive a peer-set of broken storage upstreams without wedging the tokio runtime past the k8s liveness kill window — by circuit-breaking known-broken upstreams, bounding total warm-up work, and never self-partitioning to zero upstreams.

**Architecture:** The 2026-06-13 doorway freeze is serial, CPU-bound warm-up churn against ~10 broken upstreams: `spawn_stream_task` iterates `STORAGE_URLS` one at a time, each peer burning up to 5 retries with a 10/20/40/80s backoff ladder and a 300s per-stream reqwest timeout, while the byte-stream projection loop never yields — pegging the throttled 4-worker core so `/health` stops answering within its 15s probe timeout and the liveness probe SIGKILLs the pod. **A per-peer timeout cannot fix this: the cure must act at the peer-SET level** — skip circuit-open upstreams before paying their retry ladder, cap total warm-up wall-time below ~0.5× the liveness window, shrink the per-stream timeout under the kill window with margin, and yield per projected entry. The circuit-breaker is split into a pure deterministic state machine (`CircuitBreaker` in the shared `elohim-compute` crate, advanced by injected outcomes + tick, no wall-clock) and a doorway-local per-URL map + set-level budget (`WarmStreamHealth` in `warm_stream.rs`). An anti-self-partition guard guarantees the gate never reduces the upstream set to empty.

**Tech Stack:** Rust (doorway-service `doorway`, native build; `elohim-compute` shared crate), tokio, reqwest, serde_json. No new deps. Tests are inline `#[cfg(test)]` unit tests; TDD throughout.

---

## Naming / Decisions (verification forced a split)

**Circuit-breaker home — DECIDED: SPLIT across two crates (not all-doorway-local, not all-shared).** Three witnesses had to be reconciled:

- **W2** (`elohim/elohim-compute/src/peers.rs`): "the circuit-breaker belongs HERE in peers.rs (shared) … New circuit-breaker tests append to this module." `elohim-compute` is the shared crate both doorway and storage link.
- **W7**: verified and required the `elohim-compute` build/test command — which only produces work if a type lands in that crate.
- **W3 §4**: argues against reusing the *existing `PeerHealthRegistry` API* (keyed by `conductor-{i}`, signal-shaped vocabulary `record_signal`/`reconnect_attempts`/`last_signal_at`; adding storage URLs would pollute `/status`'s conductor view and `active_count()`). W3 does NOT argue against a *new, separate* type in that file.

The resolution that satisfies all three:

1. **`CircuitBreaker`** — a pure, deterministic state machine (`record_outcome`/`is_open`/`should_skip`, advanced by an injected monotonic tick, never wall-clock) lands as a **new standalone type in `elohim/elohim-compute/src/peers.rs`**. It is NOT a method on `PeerHealthRegistry` and does not touch it. Its unit tests append to that file's existing `#[cfg(test)] mod tests`. (Satisfies W2 + W7; respects W3 by not extending `PeerHealthRegistry`.)
2. **`WarmStreamHealth`** — a doorway-local per-URL map (`url -> CircuitBreaker`) plus the SET-level total-warm-up budget logic — lands in **`doorway/doorway-service/src/projection/warm_stream.rs`** and is owned by the durable `Arc<WarmupState>` on `AppState`. It never touches `PeerHealthRegistry`. (Satisfies W3.)

**Why the map lives on the durable `WarmupState` Arc, not a local in `spawn_stream_task`:** warm-up fires at startup AND on subscriber reconnect (W1 trigger points). Half-open re-admission and across-pass skip only work if per-URL breakers survive across passes. `WarmupState` is the `Arc<...>` already shared onto `AppState` (`main.rs:620`) and read by `/health/startup` — so cure-state and observable-state share one home (W6).

**Subscriber-reconnect call site is a separate path (named integration follow-on, in-plan).** The reconnect trigger (`subscriber.rs:438-440`) calls `stream_from_peer` directly on a single URL and does NOT carry `WarmupState`. Task 12 wires that path to consult the same `WarmStreamHealth` for record-outcome (so reconnect flapping doesn't bypass the breaker); the primary gate is `spawn_stream_task`.

**Derived numbers (from W5 — liveness kill window 150–225s, conservative FLOOR = 150s):**

| Constant | Value | Derivation |
|---|---|---|
| `WARMUP_STREAM_TIMEOUT_SECS` | **45** | `< kill_window_floor(150) − margin`. Replaces the 300s per-stream reqwest timeout. 45s leaves a wide margin and lets a healthy-but-large peer's catch-up dump set `has_content` before being cut (see starvation note). |
| `WARMUP_TOTAL_BUDGET_SECS` | **75** | `≈ 0.5 × kill_window_floor(150)`. Hard cap on total warm-up wall-time across the whole peer set, enforced in the loop. |
| `WARMUP_CIRCUIT_FAIL_THRESHOLD` (K) | **3** | Open after 3 consecutive failed outcomes (trips faster than the existing 5-retry ladder). |
| `WARMUP_CIRCUIT_COOLDOWN_TICKS` | **5** | Ticks (warm-up passes) a breaker stays open before half-open. Deterministic — a tick = one warm-up pass, NOT seconds. |
| `WARMUP_YIELD_EVERY_N` | **64** | `tokio::task::yield_now()` every 64 projected entries in the byte-stream loop, so warm-up cannot monopolize the runtime. |

**Starvation-margin note (do NOT "fix" the timeout back up):** `reqwest .timeout()` is whole-request (W1); the cache/stream is a finite catch-up dump. 45s can cut a healthy-but-LARGE peer mid-stream. This is acceptable ONLY because partial progress increments the counts, which sets `has_content`, which breaks the retry loop (`warm_stream.rs:300-304`) — the peer made real progress and is NOT retried. The budget (75s) is sized to let warm-up make real progress on healthy peers; the timeout shrink keeps a 105s margin under the floor so healthy-but-slow peers are not starved.

**Coordination:** A parallel thread is active in the shared worktree (arc spec + storage hygiene leak-fixes). This plan **does NOT touch conductor config or `target_arc_factor`** (the other thread owns that). Execution must be **selective-staged** (the per-task `git add` lists name exact files only) or run in an **isolated worktree**. Commit only on the shift branch; the integrator pushes.

**Out of scope (named follow-on plans, do NOT implement here):**
- **Plan B** — inbound admission + propagated backpressure (`429`/`Retry-After`).
- **Sibling** — bilateral credit/window sender-restraint.
- **Other thread** — arc-shrink / `target_arc_factor` conductor config.
- **REA actuation** — `delegates-compute` runtime mutation of knobs (knobs stay boot/env).
- **Plan D** — full automated elevate-arm poller. This plan leaves a clean seam: a `tracing::warn!` + a structured `WarmupSelfHealEvent` wherever a failed self-heal / anti-self-partition trip occurs, for later harvest.
- **Storage↔peer edge** — DHT-attested `FeedbackSignal::Quarantine` (B2). Doorway cannot reach it (W4: no dep edge, no swarm publisher, needs mishpat/qahal auth + T19 gossip wiring). Circuit state here is operational Cat C node-local: no DHT entry, no table, no coordinator fn. Follow-on: emit a `Quarantine` from the storage side when doorway's local quarantine signal is surfaced.

---

## Canonical type & function names (MUST match across ALL tasks)

| Name | Kind | Home | Signature / shape |
|---|---|---|---|
| `CircuitState` | enum | `elohim-compute/src/peers.rs` | `Closed`, `Open`, `HalfOpen` — `#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]`, `#[serde(rename_all = "kebab-case")]` (serializes `closed`/`open`/`half-open`) |
| `CircuitBreaker` | struct | `elohim-compute/src/peers.rs` | fields: `state: CircuitState`, `consecutive_failures: u32`, `opened_at_tick: Option<u64>`, `error_streak: u32`, `last_good_tick: Option<u64>`, config: `fail_threshold: u32`, `cooldown_ticks: u64` |
| `CircuitBreaker::new` | fn | `elohim-compute/src/peers.rs` | `fn new(fail_threshold: u32, cooldown_ticks: u64) -> Self` |
| `CircuitBreaker::record_outcome` | fn | `elohim-compute/src/peers.rs` | `fn record_outcome(&mut self, ok: bool, tick: u64)` |
| `CircuitBreaker::is_open` | fn | `elohim-compute/src/peers.rs` | `fn is_open(&self) -> bool` (true iff `state == Open`) |
| `CircuitBreaker::should_skip` | fn | `elohim-compute/src/peers.rs` | `fn should_skip(&mut self, tick: u64) -> bool` — advances Open→HalfOpen when cooldown elapsed; returns false in HalfOpen (admit the one trial) |
| `CircuitBreaker::state` | fn | `elohim-compute/src/peers.rs` | `fn state(&self) -> CircuitState` |
| `CircuitBreaker::error_streak` | fn | `elohim-compute/src/peers.rs` | `fn error_streak(&self) -> u32` |
| `CircuitBreaker::last_good_tick` | fn | `elohim-compute/src/peers.rs` | `fn last_good_tick(&self) -> Option<u64>` |
| `UpstreamHealth` | struct | `warm_stream.rs` | per-URL observable snapshot: `upstream: String`, `error_streak: u32`, `last_good: Option<u64>`, `circuit: CircuitState`, `skipped: bool` — `#[serde(rename_all = "camelCase")]` |
| `WarmStreamHealth` | struct | `warm_stream.rs` | `breakers: std::sync::Mutex<std::collections::HashMap<String, CircuitBreaker>>` |
| `WarmStreamHealth::new` | fn | `warm_stream.rs` | `fn new(fail_threshold: u32, cooldown_ticks: u64) -> Self` |
| `WarmStreamHealth::should_skip` | fn | `warm_stream.rs` | `fn should_skip(&self, url: &str, tick: u64) -> bool` (creates breaker on first sight) |
| `WarmStreamHealth::record_outcome` | fn | `warm_stream.rs` | `fn record_outcome(&self, url: &str, ok: bool, tick: u64)` |
| `WarmStreamHealth::gate_upstreams` | fn | `warm_stream.rs` | `fn gate_upstreams(&self, urls: &[String], tick: u64) -> Vec<String>` — filter-before-act; **never returns empty if `urls` non-empty** (anti-self-partition) |
| `WarmStreamHealth::snapshot` | fn | `warm_stream.rs` | `fn snapshot(&self) -> Vec<UpstreamHealth>` |
| `WarmupSelfHealEvent` | struct | `warm_stream.rs` | structured harvest seam: `event: &'static str`, `detail: String` — `#[serde(rename_all = "camelCase")]` |
| `WARMUP_STREAM_TIMEOUT_SECS` | const | `warm_stream.rs` | `u64 = 45` |
| `WARMUP_TOTAL_BUDGET_SECS` | const | `warm_stream.rs` | `u64 = 75` |
| `WARMUP_CIRCUIT_FAIL_THRESHOLD` | const | `warm_stream.rs` | `u32 = 3` |
| `WARMUP_CIRCUIT_COOLDOWN_TICKS` | const | `warm_stream.rs` | `u64 = 5` |
| `WARMUP_YIELD_EVERY_N` | const | `warm_stream.rs` | `usize = 64` |

---

## File Structure

| File | Created/Modified | Responsibility |
|---|---|---|
| `elohim/elohim-compute/src/peers.rs` | Modified | Add `CircuitState` enum + `CircuitBreaker` pure state machine (open-after-K, cooldown, half-open one-trial) + unit tests. Does NOT touch `PeerHealthRegistry`. |
| `doorway/doorway-service/src/projection/warm_stream.rs` | Modified | Shrink per-stream timeout (45s); add `WarmStreamHealth` (per-URL breaker map + `gate_upstreams` anti-self-partition + `record_outcome` + `snapshot`); add `UpstreamHealth`, `WarmupSelfHealEvent`; add per-entry yield in byte-stream loop; add total-budget enforcement + breaker gating in `spawn_stream_task`; extend `WarmupState` with `health: WarmStreamHealth`. |
| `doorway/doorway-service/src/routes/health.rs` | Modified | Add `"upstreams"` array (per-URL `{upstream, errorStreak, lastGood, circuit, skipped}`) and `"budgetSecs"` to the existing `warmup` JSON block in `startup_check`. |
| `doorway/doorway-service/src/projection/subscriber.rs` | Modified | Reconnect warm-up path records its outcome into the shared `WarmStreamHealth` (Task 12). |
| `doorway/doorway-service/src/main.rs` | Modified | `WarmupState::new()` already wired (line 620); no signature change needed — `WarmStreamHealth` is a field of `WarmupState`. Verify only. |

---

## Build / test commands (verified, W7)

elohim-compute (Task 1):
```
cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo test --lib peers 2>&1 | tail -40
```

doorway-service (Tasks 2–13):
```
cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib warm_stream 2>&1 | tail -40
cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib health 2>&1 | tail -40
```

Final gate (whole crate + fmt/clippy):
```
cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib --bins 2>&1 | tail -40
cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo clippy -- -D warnings 2>&1 | tail -40
cd /projects/elohim/doorway/doorway-service && cargo fmt --check
cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo clippy -- -D warnings 2>&1 | tail -40
cd /projects/elohim/elohim/elohim-compute && cargo fmt --check
```

Rules (W7 + memory): `RUSTFLAGS=""` (both native crates), `RUSTC_WRAPPER=""` (sccache spawn-ENOENT), `/tmp` target dirs (fingerprint-ENOENT on pool slot), **plain `cargo test`, NEVER nextest**, never `&&`-pipe a gate exit code (use `2>&1 | tail -N`).

---

## TASK 1 — `CircuitBreaker` pure state machine (open-after-K + cooldown + half-open)

Files:
- `elohim/elohim-compute/src/peers.rs` — add types ABOVE `#[cfg(test)] mod tests` (insert after the `Default` impl at line 115, before line 117). Tests append to the existing `mod tests` (line 117-212), after the last test (line 211).

Pure, deterministic: tick is INJECTED (a `u64` warm-up pass counter), never wall-clock. State machine: `Closed` → (`fail_threshold` consecutive fails) → `Open` (records `opened_at_tick`) → (`should_skip` called when `tick - opened_at_tick >= cooldown_ticks`) → `HalfOpen` (admits ONE trial: `should_skip` returns false) → (trial `ok=true`) → `Closed`; (trial `ok=false`) → `Open` (re-arm `opened_at_tick = tick`).

- [ ] Write the failing test — append to `elohim-compute/src/peers.rs` `mod tests` (after line 211):
```rust
    #[test]
    fn circuit_opens_after_k_consecutive_failures() {
        let mut cb = CircuitBreaker::new(3, 5);
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
        let mut cb = CircuitBreaker::new(3, 5);
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
        // A second should_skip while HalfOpen does NOT re-admit (one trial outstanding)
        assert!(cb.should_skip(16), "half-open trial already admitted: skip until outcome recorded");
    }

    #[test]
    fn circuit_halfopen_success_closes_failure_reopens() {
        let mut cb = CircuitBreaker::new(1, 5);
        cb.record_outcome(false, 0); // open at 0
        assert!(!cb.should_skip(5)); // half-open
        cb.record_outcome(true, 6);
        assert_eq!(cb.state(), CircuitState::Closed);
        // reopen path
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
/// Advanced ONLY by injected outcomes + a monotonic `tick` (a warm-up pass
/// counter), never wall-clock — so the state machine is unit-testable without
/// time or network. Opens after `fail_threshold` consecutive failures; stays
/// open for `cooldown_ticks`; then admits exactly ONE half-open trial.
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
            CircuitState::HalfOpen => true, // trial already admitted; await outcome
            CircuitState::Open => {
                let elapsed = self
                    .opened_at_tick
                    .map(|t| tick.saturating_sub(t))
                    .unwrap_or(u64::MAX);
                if elapsed >= self.cooldown_ticks {
                    self.state = CircuitState::HalfOpen;
                    false // admit the one trial
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
- [ ] Run, expect PASS: `cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo test --lib peers 2>&1 | tail -40` — expect all peers tests pass (8 existing + 5 new = 13).
- [ ] Commit:
```
git add elohim/elohim-compute/src/peers.rs
git commit -m "feat(elohim-compute): CircuitBreaker pure state machine for upstream self-protection

Open-after-K consecutive failures, tick-injected cooldown, half-open
one-trial re-admission. Deterministic (no wall-clock). Standalone type;
does not touch PeerHealthRegistry.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 2 — Export `CircuitBreaker`/`CircuitState` from `elohim-compute` lib

Files:
- `elohim/elohim-compute/src/lib.rs` — the `peers` module is declared here (W2: `lib.rs:10,19`); ensure `CircuitBreaker`/`CircuitState` are re-exported so doorway can `use elohim_compute::peers::CircuitBreaker;` or `elohim_compute::CircuitBreaker`.

- [ ] Read `elohim/elohim-compute/src/lib.rs` and locate the existing `pub use peers::{...}` re-export line (W2 referenced `lib.rs:10,19` for `PeerHealthRegistry`/`PeerHealthSnapshot`).
- [ ] Write the failing test — add a doc-test-style assertion as a temporary unit test at the bottom of `elohim/elohim-compute/src/lib.rs` inside (or appended to) a `#[cfg(test)] mod tests`:
```rust
#[cfg(test)]
mod reexport_tests {
    #[test]
    fn circuit_breaker_is_publicly_reachable() {
        let _cb = crate::CircuitBreaker::new(3, 5);
        let _s: crate::CircuitState = crate::CircuitState::Closed;
    }
}
```
- [ ] Run, expect FAIL: `cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo test --lib reexport 2>&1 | tail -20` — expect `cannot find type CircuitBreaker in crate root`.
- [ ] Write minimal implementation — extend the existing `pub use peers::{...};` line in `lib.rs` to include `CircuitBreaker, CircuitState` (add to the brace list; if no such line exists, add `pub use peers::{CircuitBreaker, CircuitState};`).
- [ ] Run, expect PASS: `cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo test --lib reexport 2>&1 | tail -20` — expect pass.
- [ ] Commit:
```
git add elohim/elohim-compute/src/lib.rs
git commit -m "feat(elohim-compute): re-export CircuitBreaker and CircuitState

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 3 — Shrink per-stream timeout 300s → 45s

Files:
- `doorway/doorway-service/src/projection/warm_stream.rs:19-26` (constant block) and `:97-99` (the reqwest client builder).

- [ ] Write the failing test — append to `warm_stream.rs` `mod tests` (after line 357 `use super::*;`):
```rust
    #[test]
    fn per_stream_timeout_under_liveness_window() {
        // kill-window floor is 150s; per-stream timeout must stay well under it.
        assert_eq!(WARMUP_STREAM_TIMEOUT_SECS, 45);
        assert!(WARMUP_STREAM_TIMEOUT_SECS < 150 - 60, "must keep >=60s margin under floor");
    }
```
- [ ] Run, expect FAIL: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib warm_stream 2>&1 | tail -30` — expect `cannot find value WARMUP_STREAM_TIMEOUT_SECS`.
- [ ] Write minimal implementation — add to the constant block (after line 26):
```rust
/// Per-stream whole-request timeout. Shrunk from 300s so one upstream cannot
/// burn through the k8s liveness kill window (W5 floor 150s). A healthy-but-
/// large peer cut at this bound still sets `has_content`, breaking its retry
/// loop (no re-stream), so the shrink does not starve healthy peers.
const WARMUP_STREAM_TIMEOUT_SECS: u64 = 45;
```
  Then replace `warm_stream.rs:98` `.timeout(std::time::Duration::from_secs(300)) // 5 minute timeout for full stream` with `.timeout(std::time::Duration::from_secs(WARMUP_STREAM_TIMEOUT_SECS))`.
- [ ] Run, expect PASS: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib warm_stream 2>&1 | tail -30` — expect pass.
- [ ] Commit:
```
git add doorway/doorway-service/src/projection/warm_stream.rs
git commit -m "fix(doorway): shrink warm-up per-stream timeout 300s -> 45s under liveness window

Per-peer 300s timeout exceeded the 150-225s liveness kill window (W5).
45s keeps a 105s margin; partial progress sets has_content so healthy
peers are not re-streamed.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 4 — Add warm-up budget/circuit constants

Files:
- `doorway/doorway-service/src/projection/warm_stream.rs:19-26` (constant block).

- [ ] Write the failing test — append to `warm_stream.rs` `mod tests`:
```rust
    #[test]
    fn warmup_budget_constants_sane() {
        assert_eq!(WARMUP_TOTAL_BUDGET_SECS, 75); // ~0.5x kill-window floor 150
        assert!(WARMUP_TOTAL_BUDGET_SECS <= 150 / 2);
        assert_eq!(WARMUP_CIRCUIT_FAIL_THRESHOLD, 3);
        assert_eq!(WARMUP_CIRCUIT_COOLDOWN_TICKS, 5);
        assert_eq!(WARMUP_YIELD_EVERY_N, 64);
    }
```
- [ ] Run, expect FAIL: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib warm_stream 2>&1 | tail -30` — expect `cannot find value WARMUP_TOTAL_BUDGET_SECS`.
- [ ] Write minimal implementation — add to the constant block:
```rust
/// Hard cap on TOTAL warm-up wall-time across the whole peer set (~0.5x the
/// W5 liveness kill-window floor of 150s). The peer-SET bound the per-peer
/// timeout cannot provide.
const WARMUP_TOTAL_BUDGET_SECS: u64 = 75;

/// Consecutive failed warm-up outcomes before an upstream circuit opens.
const WARMUP_CIRCUIT_FAIL_THRESHOLD: u32 = 3;

/// Warm-up passes (ticks) an open circuit waits before a half-open trial.
const WARMUP_CIRCUIT_COOLDOWN_TICKS: u64 = 5;

/// Yield to the runtime every N projected entries so warm-up cannot
/// monopolize the throttled tokio workers.
const WARMUP_YIELD_EVERY_N: usize = 64;
```
- [ ] Run, expect PASS: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib warm_stream 2>&1 | tail -30` — expect pass.
- [ ] Commit:
```
git add doorway/doorway-service/src/projection/warm_stream.rs
git commit -m "feat(doorway): warm-up budget + circuit + yield constants

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 5 — `UpstreamHealth` + `WarmupSelfHealEvent` observable types

Files:
- `doorway/doorway-service/src/projection/warm_stream.rs` — add near `StreamResult` (after line 89). Import `CircuitState` at top (line 12-17 region): `use elohim_compute::{CircuitBreaker, CircuitState};`.

- [ ] Write the failing test — append to `warm_stream.rs` `mod tests`:
```rust
    #[test]
    fn upstream_health_serializes_camel_case() {
        let h = UpstreamHealth {
            upstream: "http://peer-a:8090".to_string(),
            error_streak: 4,
            last_good: Some(2),
            circuit: CircuitState::Open,
            skipped: true,
        };
        let json = serde_json::to_string(&h).unwrap();
        assert!(json.contains("\"upstream\""));
        assert!(json.contains("\"errorStreak\""));
        assert!(json.contains("\"lastGood\""));
        assert!(json.contains("\"circuit\":\"open\""));
        assert!(json.contains("\"skipped\":true"));
    }

    #[test]
    fn self_heal_event_serializes_camel_case() {
        let e = WarmupSelfHealEvent {
            event: "anti-self-partition",
            detail: "all upstreams circuit-open; proceeding least-bad".to_string(),
        };
        let json = serde_json::to_string(&e).unwrap();
        assert!(json.contains("\"event\":\"anti-self-partition\""));
        assert!(json.contains("\"detail\""));
    }
```
- [ ] Run, expect FAIL: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib warm_stream 2>&1 | tail -30` — expect `cannot find type UpstreamHealth`.
- [ ] Write minimal implementation — add the import to the `use` block and these types after `StreamResult` (line 89):
```rust
/// Per-upstream observable health surfaced on /health/startup (W6).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpstreamHealth {
    pub upstream: String,
    pub error_streak: u32,
    pub last_good: Option<u64>,
    pub circuit: CircuitState,
    pub skipped: bool,
}

/// Structured harvest seam for a failed self-heal / anti-self-partition trip.
/// Plan D (elevate-arm poller) will consume these; here we only emit + warn.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WarmupSelfHealEvent {
    pub event: &'static str,
    pub detail: String,
}
```
- [ ] Run, expect PASS: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib warm_stream 2>&1 | tail -30` — expect pass.
- [ ] Commit:
```
git add doorway/doorway-service/src/projection/warm_stream.rs
git commit -m "feat(doorway): UpstreamHealth + WarmupSelfHealEvent observable types

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 6 — `WarmStreamHealth` map: `record_outcome` + `should_skip` + `snapshot`

Files:
- `doorway/doorway-service/src/projection/warm_stream.rs` — add struct + impl after the types from Task 5.

Mirrors the storage-side `peer_statuses` consumer idiom (W3, `peer_selection.rs:138`): a map of per-key health consulted before acting. Interior mutability via `Mutex<HashMap>` because `WarmupState` is shared behind `Arc` and warm-up holds `&` not `&mut`.

- [ ] Write the failing test — append to `warm_stream.rs` `mod tests`:
```rust
    #[test]
    fn warm_stream_health_skips_after_threshold() {
        let h = WarmStreamHealth::new(WARMUP_CIRCUIT_FAIL_THRESHOLD, WARMUP_CIRCUIT_COOLDOWN_TICKS);
        let url = "http://broken:8090";
        for tick in 0..WARMUP_CIRCUIT_FAIL_THRESHOLD as u64 {
            assert!(!h.should_skip(url, tick), "closed before threshold");
            h.record_outcome(url, false, tick);
        }
        // Now open: next pass within cooldown skips.
        assert!(h.should_skip(url, WARMUP_CIRCUIT_FAIL_THRESHOLD as u64));
    }

    #[test]
    fn warm_stream_health_snapshot_reports_fields() {
        let h = WarmStreamHealth::new(1, 5);
        h.record_outcome("http://a:8090", false, 0); // opens
        h.record_outcome("http://b:8090", true, 0);  // good
        let mut snap = h.snapshot();
        snap.sort_by(|x, y| x.upstream.cmp(&y.upstream));
        assert_eq!(snap.len(), 2);
        assert_eq!(snap[0].circuit, CircuitState::Open);
        assert_eq!(snap[1].circuit, CircuitState::Closed);
        assert_eq!(snap[1].last_good, Some(0));
    }
```
- [ ] Run, expect FAIL: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib warm_stream 2>&1 | tail -30` — expect `cannot find type WarmStreamHealth`.
- [ ] Write minimal implementation — add after the Task 5 types:
```rust
/// Doorway-local per-upstream circuit map + observable snapshot. Cat C
/// node-local operational state (no DHT entry, no table). Mirrors the
/// storage-side peer_statuses consumer idiom (peer_selection.rs:138):
/// consult per-key health before acting.
pub struct WarmStreamHealth {
    breakers: std::sync::Mutex<std::collections::HashMap<String, CircuitBreaker>>,
    fail_threshold: u32,
    cooldown_ticks: u64,
}

impl WarmStreamHealth {
    pub fn new(fail_threshold: u32, cooldown_ticks: u64) -> Self {
        Self {
            breakers: std::sync::Mutex::new(std::collections::HashMap::new()),
            fail_threshold,
            cooldown_ticks,
        }
    }

    pub fn should_skip(&self, url: &str, tick: u64) -> bool {
        let mut map = self.breakers.lock().unwrap();
        let cb = map
            .entry(url.to_string())
            .or_insert_with(|| CircuitBreaker::new(self.fail_threshold, self.cooldown_ticks));
        cb.should_skip(tick)
    }

    pub fn record_outcome(&self, url: &str, ok: bool, tick: u64) {
        let mut map = self.breakers.lock().unwrap();
        let cb = map
            .entry(url.to_string())
            .or_insert_with(|| CircuitBreaker::new(self.fail_threshold, self.cooldown_ticks));
        cb.record_outcome(ok, tick);
    }

    pub fn snapshot(&self) -> Vec<UpstreamHealth> {
        let map = self.breakers.lock().unwrap();
        map.iter()
            .map(|(url, cb)| UpstreamHealth {
                upstream: url.clone(),
                error_streak: cb.error_streak(),
                last_good: cb.last_good_tick(),
                circuit: cb.state(),
                skipped: cb.is_open(),
            })
            .collect()
    }
}

impl Default for WarmStreamHealth {
    fn default() -> Self {
        Self::new(WARMUP_CIRCUIT_FAIL_THRESHOLD, WARMUP_CIRCUIT_COOLDOWN_TICKS)
    }
}
```
- [ ] Run, expect PASS: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib warm_stream 2>&1 | tail -30` — expect pass.
- [ ] Commit:
```
git add doorway/doorway-service/src/projection/warm_stream.rs
git commit -m "feat(doorway): WarmStreamHealth per-upstream breaker map (record/skip/snapshot)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 7 — `gate_upstreams` with anti-self-partition guard (NEVER empty)

Files:
- `doorway/doorway-service/src/projection/warm_stream.rs` — add method to `impl WarmStreamHealth`.

The HARD CONSTRAINT: filter circuit-open upstreams, but if filtering leaves zero AND the input was non-empty, return the least-bad upstream (lowest `error_streak`, ties → first by input order) and emit a `WarmupSelfHealEvent` + `tracing::warn!`. Tested explicitly.

- [ ] Write the failing test — append to `warm_stream.rs` `mod tests`:
```rust
    #[test]
    fn gate_filters_open_keeps_closed() {
        let h = WarmStreamHealth::new(1, 100);
        let urls = vec!["http://a".to_string(), "http://b".to_string()];
        h.record_outcome("http://a", false, 0); // a opens
        let gated = h.gate_upstreams(&urls, 1);
        assert_eq!(gated, vec!["http://b".to_string()], "open peer filtered, closed kept");
    }

    #[test]
    fn gate_never_self_partitions_to_empty() {
        let h = WarmStreamHealth::new(1, 1_000_000);
        let urls = vec!["http://a".to_string(), "http://b".to_string()];
        // Open BOTH with different streaks: a worse (3 fails), b less-bad (1 fail).
        h.record_outcome("http://a", false, 0);
        h.record_outcome("http://a", false, 1);
        h.record_outcome("http://a", false, 2);
        h.record_outcome("http://b", false, 0);
        let gated = h.gate_upstreams(&urls, 3);
        assert_eq!(gated.len(), 1, "must NOT be empty when all open");
        assert_eq!(gated[0], "http://b", "least-bad (lowest error_streak) chosen");
    }

    #[test]
    fn gate_empty_input_returns_empty() {
        let h = WarmStreamHealth::new(1, 5);
        assert!(h.gate_upstreams(&[], 0).is_empty());
    }
```
- [ ] Run, expect FAIL: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib warm_stream 2>&1 | tail -30` — expect `no method named gate_upstreams`.
- [ ] Write minimal implementation — add to `impl WarmStreamHealth`:
```rust
    /// Filter the upstream set to non-skipped peers BEFORE iterating
    /// (mirrors peer_selection.rs:138 filter-before-act). ANTI-SELF-PARTITION:
    /// if every upstream is circuit-open this returns the least-bad ONE
    /// (lowest error_streak, ties resolved by input order) rather than empty,
    /// and emits an elevate-worthy self-heal event (Plan D harvest seam).
    pub fn gate_upstreams(&self, urls: &[String], tick: u64) -> Vec<String> {
        if urls.is_empty() {
            return Vec::new();
        }
        let admitted: Vec<String> = urls
            .iter()
            .filter(|u| !self.should_skip(u, tick))
            .cloned()
            .collect();
        if !admitted.is_empty() {
            return admitted;
        }
        // All skipped: choose least-bad, never partition to none.
        let map = self.breakers.lock().unwrap();
        let least_bad = urls
            .iter()
            .min_by_key(|u| map.get(*u).map(|cb| cb.error_streak()).unwrap_or(0))
            .cloned()
            .unwrap_or_else(|| urls[0].clone());
        drop(map);
        let event = WarmupSelfHealEvent {
            event: "anti-self-partition",
            detail: format!(
                "all {} upstreams circuit-open; proceeding least-bad {}",
                urls.len(),
                least_bad
            ),
        };
        warn!(
            event = event.event,
            detail = %event.detail,
            "Warm-up anti-self-partition guard tripped (elevate-worthy)"
        );
        vec![least_bad]
    }
```
- [ ] Run, expect PASS: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib warm_stream 2>&1 | tail -30` — expect pass.
- [ ] Commit:
```
git add doorway/doorway-service/src/projection/warm_stream.rs
git commit -m "feat(doorway): gate_upstreams with anti-self-partition guard (never empty)

All-circuits-open proceeds least-bad + emits WarmupSelfHealEvent + warn!
(Plan D harvest seam). Mirrors peer_selection.rs:138 filter-before-act.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 8 — Attach `WarmStreamHealth` to `WarmupState`

Files:
- `doorway/doorway-service/src/projection/warm_stream.rs:237-261` (`WarmupState` struct + `new`).

- [ ] Write the failing test — append to `warm_stream.rs` `mod tests`:
```rust
    #[test]
    fn warmup_state_has_health_map() {
        let ws = WarmupState::new();
        ws.health.record_outcome("http://x:8090", true, 0);
        let snap = ws.health.snapshot();
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].upstream, "http://x:8090");
    }
```
- [ ] Run, expect FAIL: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib warm_stream 2>&1 | tail -30` — expect `no field health on type WarmupState`.
- [ ] Write minimal implementation — add the field to `WarmupState` (after line 242 `completed`):
```rust
    pub health: WarmStreamHealth,
```
  and to `WarmupState::new` (after line 252 `completed: AtomicBool::new(false),`):
```rust
            health: WarmStreamHealth::new(
                WARMUP_CIRCUIT_FAIL_THRESHOLD,
                WARMUP_CIRCUIT_COOLDOWN_TICKS,
            ),
```
- [ ] Run, expect PASS: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib warm_stream 2>&1 | tail -30` — expect pass.
- [ ] Commit:
```
git add doorway/doorway-service/src/projection/warm_stream.rs
git commit -m "feat(doorway): attach WarmStreamHealth to durable WarmupState

Per-URL breakers survive across warm-up passes (startup + reconnect).

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 9 — Per-entry yield in the byte-stream projection loop

Files:
- `doorway/doorway-service/src/projection/warm_stream.rs:135-224` (the `while let Some(chunk_result)` loop; the per-entry counter at 208-214).

The byte-stream loop has NO yield between projected entries — it pegs the throttled core (advisor point 4; W1). Add a counter that calls `tokio::task::yield_now().await` every `WARMUP_YIELD_EVERY_N` projected entries. This task has no pure unit test (it requires an async stream); it is verified by compilation + the final whole-crate gate, with the logic asserted via a tiny helper test.

- [ ] Write the failing test — append to `warm_stream.rs` `mod tests` (asserts the modulo gate logic via a free helper):
```rust
    #[test]
    fn yield_gate_fires_every_n() {
        fn should_yield(n: usize) -> bool {
            n % WARMUP_YIELD_EVERY_N == 0
        }
        assert!(should_yield(WARMUP_YIELD_EVERY_N));
        assert!(should_yield(WARMUP_YIELD_EVERY_N * 2));
        assert!(!should_yield(1));
        assert!(!should_yield(WARMUP_YIELD_EVERY_N - 1));
    }
```
- [ ] Run, expect FAIL: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib warm_stream 2>&1 | tail -30` — expect compile pass but this test currently absent; if `WARMUP_YIELD_EVERY_N` missing it fails (it exists from Task 4, so this confirms the test compiles). The behavioral FAIL is the missing yield in the loop — verified by reading line 224 (no `yield_now` present).
- [ ] Write minimal implementation — in `stream_from_peer`, add a projected-entry counter. Before the `while let Some(chunk_result)` loop (after line 133 `let mut event_lines: Vec<String> = Vec::new();`) add:
```rust
    let mut projected: usize = 0;
```
  Then in the success arm where counts increment (inside the `else` block at lines 208-215, after the `match doc_type { ... }`), add:
```rust
                                projected += 1;
                                if projected % WARMUP_YIELD_EVERY_N == 0 {
                                    tokio::task::yield_now().await;
                                }
```
- [ ] Run, expect PASS: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib warm_stream 2>&1 | tail -30` — expect pass; confirm compile clean.
- [ ] Commit:
```
git add doorway/doorway-service/src/projection/warm_stream.rs
git commit -m "fix(doorway): yield runtime every N projected entries in warm-up

Byte-stream projection loop had no yield; pegged the throttled core so
/health stopped answering (W5 freeze mechanism).

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 10 — Gate `spawn_stream_task` on breakers + enforce total budget + record outcomes

Files:
- `doorway/doorway-service/src/projection/warm_stream.rs:269-354` (`spawn_stream_task`).

Wire all the pieces into the loop. Signature change: `spawn_stream_task` takes the `Option<Arc<WarmupState>>` it already has (line 273) — the health map rides on it. Behavior: (a) compute a `tick` (one per full pass; for the startup single-pass call use `tick = 0`, but increment per outer retry-of-the-set if added later — here pass `0`); (b) `let gated = ws.health.gate_upstreams(&storage_urls, tick)` BEFORE the `for` (anti-self-partition: gated is never empty if input non-empty); (c) iterate `gated` instead of `storage_urls`; (d) inside the inner loop, after `stream_from_peer`, call `ws.health.record_outcome(url, ok, tick)` where `ok = result.errors.is_empty() || has_content`; (e) if `record_outcome` opens the breaker (`ws.health.should_skip(url, tick)` true), BREAK the inner retry loop early (stop burning the 10/20/40/80s ladder on a now-broken peer — advisor point 2b); (f) wrap the whole pass in a `tokio::time::timeout(Duration::from_secs(WARMUP_TOTAL_BUDGET_SECS), ...)` and on elapse emit a `WarmupSelfHealEvent{event:"budget-exhausted",...}` + `warn!` and stop (advisor point 3: orthogonal to anti-self-partition).

- [ ] Write the failing test — append to `warm_stream.rs` `mod tests` (asserts the gate-then-iterate contract via the health map, since `spawn_stream_task` itself needs a live store; this asserts the early-bail decision logic the loop uses):
```rust
    #[test]
    fn open_breaker_bails_inner_retry_early() {
        // Simulate the inner-loop decision: after K failures recorded, the
        // loop must observe is_open and stop, not keep retrying.
        let h = WarmStreamHealth::new(WARMUP_CIRCUIT_FAIL_THRESHOLD, 100);
        let url = "http://slowbroken:8090";
        let mut bailed_at = None;
        for attempt in 1..=MAX_WARMUP_RETRIES {
            h.record_outcome(url, false, 0);
            if h.should_skip(url, 0) {
                bailed_at = Some(attempt);
                break;
            }
        }
        assert_eq!(
            bailed_at,
            Some(WARMUP_CIRCUIT_FAIL_THRESHOLD),
            "must bail at K, not run all {MAX_WARMUP_RETRIES} retries"
        );
    }
```
- [ ] Run, expect FAIL: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib warm_stream 2>&1 | tail -30` — expect pass-compile but assert that current `spawn_stream_task` does NOT gate/record (verified by reading 288-347: no `gate_upstreams`/`record_outcome`/`timeout`). This test passes on the map alone; the loop wiring is verified structurally + by the whole-crate gate.
- [ ] Write minimal implementation — rewrite the `tokio::spawn(async move { ... })` body of `spawn_stream_task` (lines 275-353) to:
```rust
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;

        if let Some(ref ws) = warmup_state {
            ws.in_progress.store(true, Ordering::Relaxed);
        }

        let tick: u64 = 0; // single startup pass; reconnect passes increment via subscriber path

        // Gate the upstream set BEFORE iterating (filter-before-act); the guard
        // guarantees a non-empty result if storage_urls is non-empty.
        let gated: Vec<String> = match warmup_state {
            Some(ref ws) => ws.health.gate_upstreams(&storage_urls, tick),
            None => storage_urls.clone(),
        };

        info!(
            peer_count = gated.len(),
            total_peers = storage_urls.len(),
            "Starting cache stream warm-up (health-gated)"
        );

        let budget = std::time::Duration::from_secs(WARMUP_TOTAL_BUDGET_SECS);
        let pass = async {
            for storage_url in &gated {
                let mut attempt: u32 = 0;
                loop {
                    attempt += 1;
                    if let Some(ref ws) = warmup_state {
                        ws.attempts.store(attempt, Ordering::Relaxed);
                    }

                    let result = stream_from_peer(Arc::clone(&store), storage_url).await;
                    let has_content = result.content_count > 0
                        || result.human_count > 0
                        || result.relationship_count > 0;
                    let ok = result.errors.is_empty() || has_content;

                    if let Some(ref ws) = warmup_state {
                        ws.health.record_outcome(storage_url, ok, tick);
                    }

                    if ok {
                        info!(
                            storage_url = %storage_url,
                            content = result.content_count,
                            humans = result.human_count,
                            relationships = result.relationship_count,
                            attempt,
                            "Cache stream warm-up completed successfully"
                        );
                        break;
                    }

                    if let Some(ref ws) = warmup_state {
                        if let Ok(mut guard) = ws.last_error.lock() {
                            *guard = result.errors.first().cloned();
                        }
                        // Breaker opened mid-pass: stop burning the backoff ladder.
                        if ws.health.should_skip(storage_url, tick) {
                            warn!(
                                storage_url = %storage_url,
                                attempt,
                                "Upstream circuit opened; bailing retry ladder early"
                            );
                            break;
                        }
                    }

                    if attempt >= MAX_WARMUP_RETRIES {
                        error!(
                            storage_url = %storage_url,
                            attempts = attempt,
                            errors = ?result.errors,
                            "Cache stream warm-up failed after max retries"
                        );
                        break;
                    }

                    let retry_delay = WARMUP_RETRY_BASE_SECS
                        .saturating_mul(2u64.pow(attempt - 1))
                        .min(WARMUP_RETRY_MAX_SECS);
                    warn!(
                        storage_url = %storage_url,
                        attempt,
                        max_retries = MAX_WARMUP_RETRIES,
                        retry_delay_secs = retry_delay,
                        errors = ?result.errors,
                        "Cache stream warm-up failed, retrying"
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(retry_delay)).await;
                }
            }
        };

        if tokio::time::timeout(budget, pass).await.is_err() {
            let event = WarmupSelfHealEvent {
                event: "budget-exhausted",
                detail: format!("warm-up exceeded {WARMUP_TOTAL_BUDGET_SECS}s total budget"),
            };
            warn!(
                event = event.event,
                detail = %event.detail,
                "Warm-up total budget exhausted; stopping pass (elevate-worthy)"
            );
        }

        if let Some(ref ws) = warmup_state {
            ws.in_progress.store(false, Ordering::Relaxed);
            ws.completed.store(true, Ordering::Relaxed);
        }
    })
```
- [ ] Run, expect PASS: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib warm_stream 2>&1 | tail -30` — expect pass.
- [ ] Commit:
```
git add doorway/doorway-service/src/projection/warm_stream.rs
git commit -m "fix(doorway): health-gate + total-budget-bound warm-up peer-set loop

Skip circuit-open upstreams before paying their retry ladder; bail the
backoff ladder when a breaker opens mid-pass; cap total warm-up at 75s
(<=0.5x liveness window). Anti-self-partition guard keeps the set non-empty.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 11 — Surface per-upstream health on `/health/startup`

Files:
- `doorway/doorway-service/src/routes/health.rs:359-369` (the `warmup` JSON block in `startup_check`).

Add `"upstreams"` (array of `UpstreamHealth`, camelCase) and `"budgetSecs"` to the existing `warmup` object. No new endpoint, no new struct (W6).

- [ ] Write the failing test — `startup_check` returns a `Response`; assert the JSON body contains the new keys. Append to `health.rs`'s `#[cfg(test)] mod tests` (locate it; if absent, add one) a test that builds a minimal `AppState` with a `warmup_state` whose `health` has one recorded outcome, calls `startup_check`, and asserts the body string contains `"upstreams"` and `"budgetSecs"`. If `AppState` construction in tests is heavy, instead assert at the serialization level:
```rust
    #[test]
    fn startup_warmup_block_includes_upstreams_and_budget() {
        use doorway::projection::warm_stream::WarmupState;
        let ws = WarmupState::new();
        ws.health.record_outcome("http://peer-a:8090", false, 0);
        let upstreams = serde_json::to_value(ws.health.snapshot()).unwrap();
        let block = serde_json::json!({
            "inProgress": false,
            "upstreams": upstreams,
            "budgetSecs": 75u64,
        });
        let s = block.to_string();
        assert!(s.contains("\"upstreams\""));
        assert!(s.contains("\"errorStreak\""));
        assert!(s.contains("\"budgetSecs\":75"));
    }
```
  (If `health.rs` has no test module, add `#[cfg(test)] mod tests { use super::*; ... }` at the file end. `WarmupState` must be `pub` and reachable via the crate name `doorway` — it already is, per `main.rs:621` usage.)
- [ ] Run, expect FAIL: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib health 2>&1 | tail -30` — expect the assertion or compile to fail until the snapshot wiring exists (it does from Task 6/8, so this test passes once those land; the BEHAVIORAL change is the handler emitting the keys, verified next).
- [ ] Write minimal implementation — replace the `warmup` block (health.rs:359-369) with:
```rust
    let warmup = if let Some(ref ws) = state.warmup_state {
        serde_json::json!({
            "inProgress": ws.in_progress.load(std::sync::atomic::Ordering::Relaxed),
            "attempts": ws.attempts.load(std::sync::atomic::Ordering::Relaxed),
            "maxAttempts": ws.max_attempts.load(std::sync::atomic::Ordering::Relaxed),
            "completed": ws.completed.load(std::sync::atomic::Ordering::Relaxed),
            "lastError": ws.last_error.lock().unwrap().clone(),
            "budgetSecs": 75,
            "upstreams": ws.health.snapshot(),
        })
    } else {
        serde_json::json!(null)
    };
```
  (`UpstreamHealth` derives `Serialize`, so `ws.health.snapshot()` serializes directly inside `json!`.)
- [ ] Run, expect PASS: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib health 2>&1 | tail -30` — expect pass.
- [ ] Commit:
```
git add doorway/doorway-service/src/routes/health.rs
git commit -m "feat(doorway): surface per-upstream circuit/health on /health/startup

New warmup.upstreams[] {upstream,errorStreak,lastGood,circuit,skipped}
+ warmup.budgetSecs. No new endpoint (W6).

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 12 — Wire reconnect warm-up path to record outcomes

Files:
- `doorway/doorway-service/src/projection/subscriber.rs:423-445` (the reconnect warm-up trigger).

The reconnect path calls `stream_from_peer` directly (single URL) and bypasses the breaker. Wire it to record its outcome into the shared `WarmStreamHealth` so reconnect flapping feeds the same circuit. The subscriber must hold an `Arc<WarmupState>` (or its `health`); check `SubscriberConfig`/`self.config` for a `warmup_state` field. If absent, thread `Option<Arc<WarmupState>>` through `SubscriberConfig`.

- [ ] Read `subscriber.rs` `SubscriberConfig` (around line 300-320) and `main.rs` subscriber construction to see whether `warmup_state` is already available; confirm whether the subscriber can reach `state.warmup_state`.
- [ ] Write the failing test — this path is async/networked; add a unit test asserting the decision: append to `subscriber.rs` `#[cfg(test)] mod tests` (or create one) a test confirming that given a `WarmStreamHealth`, recording a reconnect failure increments the streak (the integration is verified by the whole-crate gate):
```rust
    #[test]
    fn reconnect_failure_feeds_shared_breaker() {
        use crate::projection::warm_stream::WarmStreamHealth;
        let h = WarmStreamHealth::new(3, 5);
        h.record_outcome("ws://conductor:8445", false, 0);
        let snap = h.snapshot();
        assert_eq!(snap[0].error_streak, 1);
    }
```
- [ ] Run, expect FAIL: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib subscriber 2>&1 | tail -30` — expect pass-on-map if `WarmStreamHealth` reachable; if `warmup_state` not threaded the implementation step compiles it in.
- [ ] Write minimal implementation — if `SubscriberConfig` lacks `warmup_state`, add `pub warmup_state: Option<Arc<crate::projection::warm_stream::WarmupState>>,` to it and populate from `state.warmup_state` at construction in `main.rs`. Then in the reconnect spawn (subscriber.rs:439-442), capture the URL outcome:
```rust
                    let store = Arc::clone(store);
                    let url = storage_url.clone();
                    let active = Arc::clone(&self.warm_stream_active);
                    let ws = self.config.warmup_state.clone();
                    tokio::spawn(async move {
                        let result = super::warm_stream::stream_from_peer(store, &url).await;
                        let ok = result.errors.is_empty()
                            || result.content_count + result.human_count + result.relationship_count > 0;
                        if let Some(ws) = ws {
                            ws.health.record_outcome(&url, ok, 0);
                        }
                        active.store(false, std::sync::atomic::Ordering::SeqCst);
                    });
```
- [ ] Run, expect PASS: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib subscriber 2>&1 | tail -30` — expect pass; confirm `main.rs` compiles with the new config field.
- [ ] Commit:
```
git add doorway/doorway-service/src/projection/subscriber.rs doorway/doorway-service/src/main.rs
git commit -m "feat(doorway): reconnect warm-up records outcomes into shared breaker map

Reconnect flapping now feeds the same WarmStreamHealth so it cannot
bypass the circuit.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 13 — Whole-crate green gate + fmt + clippy (both crates)

Files: none new — verification only.

- [ ] Run doorway whole-crate tests: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib --bins 2>&1 | tail -40` — expect 331+ tests pass (W-quote: "331+").
- [ ] Run doorway clippy: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo clippy -- -D warnings 2>&1 | tail -40` — expect no warnings.
- [ ] Run doorway fmt: `cd /projects/elohim/doorway/doorway-service && cargo fmt --check` — expect clean (fix with `cargo fmt` if not, then re-run).
- [ ] Run elohim-compute tests + clippy + fmt: `cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo test --lib 2>&1 | tail -40` then `... cargo clippy -- -D warnings 2>&1 | tail -40` then `cargo fmt --check` — expect all clean.
- [ ] If any fmt fixes were applied, commit them:
```
git add elohim/elohim-compute/src/peers.rs doorway/doorway-service/src/projection/warm_stream.rs doorway/doorway-service/src/routes/health.rs doorway/doorway-service/src/projection/subscriber.rs
git commit -m "style(upstream-self-protection): cargo fmt

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```
- [ ] Invoke `story-harvest` to scaffold an a2o regression scenario capturing the freeze constraint (total warm-up budget vs liveness window; circuit-break broken upstreams; never self-partition) — these are exactly the parameter-bearing discoveries story-harvest preserves.

---

## Self-Review

**Spec coverage (every IN item maps to a task):**
- (1) Circuit-breaker `record_outcome`/`is_open`/`should_skip`, open-after-K, cooldown, half-open one-trial, deterministic tick → **Task 1** (pure state machine, `elohim-compute`), home decided per W2/W7 (shared) — see Naming/Decisions.
- (2) Health-gate the warm-up (skip `is_open()` upstreams; stop burning backoff) → **Task 7** (`gate_upstreams`) + **Task 10** (gate before loop + bail ladder mid-pass); mirrors `peer_selection.rs:138`.
- (3) Bound total warm-up vs liveness window (total budget ≤0.5× kill window = 75s; per-stream timeout < kill_window − margin = 45s replacing 300s; per-tick yield) → **Task 3** (timeout), **Task 4** (constants), **Task 9** (yield), **Task 10** (75s budget).
- (4) Anti-self-partition (never skip the last healthy upstream; least-bad + elevate event; tested) → **Task 7** (`gate_upstreams` guard + `gate_never_self_partitions_to_empty` test).
- (5) Expose per-upstream `{upstream, error_streak, last_good, circuit, skipped}` on the read surface (W6) → **Task 5** (`UpstreamHealth`) + **Task 6** (`snapshot`) + **Task 11** (`/health/startup` `upstreams[]`).
- OUT items (admission/429, bilateral credit, arc-shrink, REA actuation, elevate poller, FeedbackSignal::Quarantine) all named in "Out of scope"; the Plan-D harvest seam (`WarmupSelfHealEvent` + `tracing::warn!`) is implemented in Tasks 7 & 10.

**Placeholder scan:** No `TODO`, no `<...>`, no "implement here". Every step has actual test code, exact commands, actual implementation code, and a concrete `git add`/commit with the `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>` trailer.

**Type/fn-name consistency (cross-task):** `CircuitBreaker`/`CircuitState` defined Task 1, exported Task 2, consumed by `WarmStreamHealth` (Tasks 6-8), `UpstreamHealth` (Task 5). `WarmStreamHealth::{new,should_skip,record_outcome,gate_upstreams,snapshot}` — `new` (T6), `should_skip`/`record_outcome` (T6), `gate_upstreams` (T7), `snapshot` (T6) — all match the canonical names table and are consumed identically in Tasks 8/10/11/12. Constants `WARMUP_STREAM_TIMEOUT_SECS`(45)/`WARMUP_TOTAL_BUDGET_SECS`(75)/`WARMUP_CIRCUIT_FAIL_THRESHOLD`(3)/`WARMUP_CIRCUIT_COOLDOWN_TICKS`(5)/`WARMUP_YIELD_EVERY_N`(64) defined once (T3/T4), referenced by exact name in T6/T8/T9/T10. `UpstreamHealth` field serialization (`errorStreak`/`lastGood`/`circuit`/`skipped`) is camelCase per W6 and asserted in T5. `WarmupSelfHealEvent` emitted in T7 (anti-self-partition) and T10 (budget-exhausted) with matching shape.

**Determinism check:** `CircuitBreaker` advances only via injected `(ok, tick)` — no `Instant::now`/`SystemTime` in T1 (the only wall-clock is `tokio::time::timeout` in T10's runtime path, deliberately outside the pure state machine). Tests in T1/T6/T7 use fixed tick values; no network, no sleep.

**Scar adherence:** per-peer timeout explicitly stated insufficient (Architecture + Task 10); starvation-margin note guards against re-inflating the timeout; circuit state declared Cat C node-local (no DHT/table/coordinator fn); FeedbackSignal::Quarantine a named follow-on; conductor config / `target_arc_factor` untouched (other thread); selective-staged commits (each `git add` names exact files).
