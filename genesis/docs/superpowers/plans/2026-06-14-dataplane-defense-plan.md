# Dataplane Defense — Fix-Patterns Over Already-Built (Implementation Plan)

> For agentic workers: REQUIRED SUB-SKILL: superpowers:subagent-driven-development (or executing-plans). Steps use checkbox (- [ ]) syntax. Working draft — NOT cite-sealed.

**Track:** P-DEFENSE (one of seven cohesive P2P-dataplane plans, 2026-06-14). Authored against `/projects/elohim/P2P-DATAPLANE-CONTRACT-LEDGER-2026-06-14.md` — obey its file-ownership, single-owners, interface names, and dependency edges exactly.

**Goal:** Close the *residual* no-overwhelm / herd-resistance gaps that survive the already-built A/B/C/D self-healing control plane — the small fix-patterns that the bigger pieces left behind. Concretely: (1) put **jitter** in both conductor-reconnect paths so N nodes do not reconnect in lockstep after a conductor blip (thundering herd); (2) give storage's conductor client real **backoff** (it has a fixed 5s sleep, no growth, no jitter); (3) extract a single shared `jittered()` primitive into `elohim-compute` so both consumers share one definition (the CircuitBreaker single-owner precedent); (4) delete the dead `rate_limit_rpm` config-theater field; (5) migrate the lower-traffic per-request `reqwest::Client::new()` routes onto the already-pooled shared client (residual wedge class beyond the production hot path, which is already pooled); (6) **DIAGNOSE** the undiagnosed doorway render wedge (settled pod, ~2.5min, zero churn) by instrumenting the single-slot `sync_channel(1)` per V8 isolate — a diagnosis task, not a blind fix.

**Architecture context (what is ALREADY built — verify-only, do NOT re-plan):**
- `elohim-storage/src/http.rs` Pillar-2 admission shed (`MAX_CONCURRENT_REQUESTS=64` + `try_acquire`→503+`Retry-After`, `storage_admission_shed_total`). REFUTES "queues-never-sheds."
- `doorway/server/http.rs:315 init_storage_proxy_client()` builds ONE pooled, connect+request-timeout-bounded `Arc<reqwest::Client>` on `AppState.storage_proxy_client`, used at http.rs:3115/3255/3284/3296. REFUTES "unbounded hot-path Client" for the production path. The surviving `Client::new()` hits are `#[cfg(test)]` in storage_proxy.rs OR lower-traffic routes (this plan's scope).
- `doorway/worker/conductor.rs:55 ReconnectBackoff` is exponential + stability-gated (resets only after a session survives `STABLE_SESSION_THRESHOLD`) — but has **no jitter** (`next_after_connect_failure` is pure `*2`, lines 68-71).
- `doorway/services/zome_caller.rs is_transport_error()` (iteration-3, commit `89bc208a8`) gates `*ws=None` on transport class only. Present on `feat` + `shift/self-healing-control-plane`, **NOT on `dev`** (`git merge-base --is-ancestor 89bc208a8 dev` → NOT ON DEV). Residual is *integration* (push feat→dev), not new code — OUT OF SCOPE here (named in §FOLLOW-ON).
- `elohim-compute/src/lib.rs:19` already re-exports `CircuitBreaker, CircuitState` (the upstream-self-protection plan landed) — the single-owner precedent this plan mirrors for `backoff`.

**Tech stack:** Rust (doorway-service `doorway` native, elohim-storage WASM-flagged crate, elohim-compute shared native), tokio, reqwest. New dep: **`rand`** in `elohim-compute` (for jitter) — see Task 1 note. Tests are inline `#[cfg(test)]` unit tests; TDD throughout.

---

## 1. CONTEXT — findings this plan closes

| Ledger / review finding | Verdict (verified file:line) | Closed by task |
|---|---|---|
| conductor_client fixed-5s, no backoff, no jitter (storage herd) | CONFIRMED `conductor_client.rs:57 reconnect_delay: Duration::from_secs(5)`; fixed sleep at 232/251/334 | T2, T3 |
| doorway ReconnectBackoff exponential but no jitter | CONFIRMED `worker/conductor.rs:68-71` pure `*2` | T4 |
| No shared backoff primitive (drift risk) | CONFIRMED no `elohim-compute/src/backoff.rs` | T1 (OWNS S7 `jittered`) |
| dead `rate_limit_rpm` config theater | CONFIRMED `route_registry.rs:91` threaded (`:226,:475,:550`), set to literals, **zero enforcing consumers** | T5 |
| residual per-request `Client::new()` in lower-traffic routes | CONFIRMED `threshold.rs:45, elohim_agent.rs:87, epr.rs:40, collectives.rs:144, apps.rs:224/373/572, seed.rs:278, blob.rs:493` | T6 |
| undiagnosed doorway wedge (settled, ~2.5min, zero churn) | CONFIRMED isolable: `elohim-render/src/angular.rs:112 mpsc::sync_channel::<StringWorkItem>(1)` single-slot per isolate → head-of-line serialization; render concurrency gated by isolate count, not `render_semaphore` | T7 (DIAGNOSE only) |
| `record_health_attestation` next `record_heartbeat`-class driver | NEW `federation.rs:289` every ~5th 60s tick; relies on iteration-3 keeping the conn on app-Err | T8 (regression guard) |

REFUTED (already built — do NOT touch): storage admission queues-never-sheds; unbounded hot-path storage_proxy Client; UpstreamBreakers (D6); the iteration-3 transport-vs-app classification (needs feat→dev integration, not new code).

---

## 2. OWNED FILES (verbatim from the ledger §2 P-DEFENSE block)

**Creates (C) / Mutates (M):**
- **C** `elohim/elohim-compute/src/backoff.rs` — SOLE owner.
- **M** `elohim/elohim-compute/src/lib.rs` — append-only re-export block (RESOLUTION-E; disjoint module name `backoff`).
- **M** `elohim/elohim-compute/Cargo.toml` — add `rand` dep (P-DEFENSE-local; no other plan edits elohim-compute Cargo.toml).
- **M** `elohim/elohim-storage/src/conductor_client.rs` (add backoff+jitter to reconnect_delay) — SOLE owner.
- **M** `doorway/doorway-service/src/worker/conductor.rs` (add jitter to existing ReconnectBackoff) — SOLE owner.
- **M** `doorway/doorway-service/src/services/route_registry.rs` (delete dead `rate_limit_rpm`) — RESOLUTION-I, SOLE owner (P-TRANSPORT does NOT touch registry).
- **M** `doorway/doorway-service/src/server/http.rs` (delete the two `rate_limit_rpm: 0` literals at :4183,:4260 that feed the registry struct) — see collision note.
- **M** lower-traffic route files → shared pooled client: `doorway/doorway-service/src/routes/{threshold.rs, elohim_agent.rs, epr.rs, collectives.rs, apps.rs, seed.rs, blob.rs}` — SOLE owner (these only READ `AppState.storage_proxy_client`).
- **Diagnosis only, NO mutation of behavior:** `elohim/elohim-render/src/angular.rs` (instrument the `sync_channel(1)` saturation — additive `tracing` + a counter; no semantic change).

**Collision statement:** This plan touches **no file owned by another plan as a structural mutator.** The only shared-with-others files are append-only seams resolved by the ledger: `elohim-compute/src/lib.rs` (RESOLUTION-E — append one re-export block) and `elohim-compute/Cargo.toml` (P-DEFENSE-exclusive among the seven — no other plan edits it). `route_registry.rs` is RESOLUTION-I (P-TRANSPORT's file list does NOT include it → P-DEFENSE SOLE owner). `server/http.rs` here is a 2-literal deletion tied to the `rate_limit_rpm` struct-field removal in the SAME crate the registry lives in — flagged below as a hand-off note since P-DIAGNOSTIC owns `main.rs`/`health.rs` and P-ACTUATION owns `http.rs` only in the **storage** crate (a different `http.rs`). The doorway `server/http.rs` is not in any other plan's file list; confirmed P-DEFENSE-local.

---

## 3. PRIMITIVES — OWNED and CONSUMED

### OWNED (this plan is the single owner — ledger S7)
| Name | Kind | Home | Signature |
|---|---|---|---|
| `jittered` | fn | `elohim_compute::backoff` | `pub fn jittered(base: Duration, max: Duration, attempt: u32) -> Duration` — exponential `base * 2^attempt` clamped to `max`, then full-jitter randomized in `[0, capped]` |

Ledger consumers of S7: P-RECONCILE (optional), P-ARC (optional), plus this plan's two confirmed consumers (storage `conductor_client`, doorway `worker/conductor`). This plan lands `jittered` in `elohim-compute` (zero new crate — two confirmed consumers prove the boundary).

### CONSUMED (skip-if-present clause, verbatim from ledger §1)
> *"Before landing this type, verify `elohim-compute` (or the named owner module) already exposes it. If present, VERIFY-ONLY (import + use). If absent at your integration point, land the owner-plan's verbatim definition only as a temporary local shim, flag it in your plan's hand-off notes, and delete the shim when the owner lands."*

| Consumed primitive | Owner | Use here | Skip-if-present check |
|---|---|---|---|
| `CircuitBreaker`/`CircuitState` (S8) | already shipped (`elohim_compute::peers`) | verify-only model reference (this plan does NOT redefine; not directly imported) | `grep 'pub use peers::{.*CircuitBreaker' elohim/elohim-compute/src/lib.rs` → present at :19. VERIFY-ONLY. |
| `UpstreamBreakers`/`ProxyOutcome` (S11) | already shipped (`storage_proxy.rs`) | consume verbatim where blob routes already use them; do NOT redefine | present (Plan B/D). VERIFY-ONLY. |
| `AppState.storage_proxy_client: Arc<reqwest::Client>` | already shipped (`server/http.rs:295,315`) | T6 routes consume this Arc instead of `Client::new()` | present. VERIFY-ONLY. |
| `zome_call_timeout()` / `is_transport_error()` | already shipped (`services/zome_caller.rs`) | consume; T8 guards the periodic-call inventory that relies on it | present. VERIFY-ONLY. |

**No type owned by another P-* plan is consumed by this plan.** P-DEFENSE does not consume `ActuationRefusal` (S1), `Sweep` (S5), `SweepRegistrySnapshot` (S5), `connection_limits` (S14), or `P2PStatusInfo` fields (S9). It is the most independent plan.

---

## 4. DEPENDENCY EDGES (ledger §4 DAG)

- **P-DEFENSE → (none).** Fully independent root; owns S7; diagnosis-only on `angular.rs`. Zero inbound HARD edges from this plan onto others; zero outbound consumers blocking it.
- **(soft, inbound) P-RECONCILE → P-DEFENSE** and **P-ARC → P-DEFENSE** via S7 `jittered` — those plans MAY consume `jittered`; if this plan slips, they hold a local shim. **This plan does not wait on them.**
- **Dispatch wave: WAVE 1 (root).** Runs fully in parallel with P-ACTUATION, P-RECONCILE, P-PROOFS-core. No file-sequencing against any other plan.

---

## 5. CANONICAL NAMES (must match across all tasks)

| Name | Kind | Home | Shape |
|---|---|---|---|
| `jittered` | fn | `elohim_compute::backoff` | `pub fn jittered(base: Duration, max: Duration, attempt: u32) -> Duration` |
| `BackoffLadder` | struct (private helper) | `elohim_compute::backoff` | `{ base: Duration, max: Duration, attempt: u32 }` + `next(&mut self) -> Duration` (uses `jittered`); convenience for stateful callers. Internal; not re-exported unless a consumer needs it. |
| `STORAGE_RECONNECT_BASE` / `STORAGE_RECONNECT_MAX` | const | `conductor_client.rs` | `Duration` = 1s / 60s — replaces the fixed 5s |
| (existing) `BASE_RECONNECT_DELAY` / `MAX_RECONNECT_DELAY` | const | `worker/conductor.rs:42,45` | UNCHANGED (100ms / 30s); T4 only adds jitter to the *returned* delay |

---

## 6. P2P-CLASS OF NEW ENTITIES (p2p-design-gate — cite the class, do not re-litigate)

Per shared grounding: new runtime entities here are **Cat-C node-local read-models / operational state** — no DHT entry type, no table, no coordinator fn, no signal. `jittered` is a pure arithmetic helper (no entity). Backoff/jitter state (`ReconnectBackoff`, `BackoffLadder`) is per-process operational state on a single node's reconnect loop — Cat-C. The `rate_limit_rpm` deletion removes config theater (no entity). The route-client migration changes which `reqwest::Client` instance is used (no entity). The `angular.rs` instrumentation is diagnostic telemetry (Cat-C, surfaces via the existing render-stats / `x-ssr-*` path — NOT a new endpoint). **No Cat-A/A2/B/B2 entity is created.**

---

## Build / test commands (per-crate RUSTFLAGS, /tmp target, plain cargo test)

elohim-compute (Tasks 1):
```
cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo test --lib backoff 2>&1 | tail -40
```
elohim-storage (Tasks 2-3) — **WASM getrandom flag REQUIRED for this crate**:
```
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo test --lib conductor_client 2>&1 | tail -40
```
doorway-service (Tasks 4-8) — **RUSTFLAGS="" REQUIRED (native)**:
```
cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib conductor 2>&1 | tail -40
cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib route_registry 2>&1 | tail -40
```
elohim-render (Task 7 — native, no special flags; diagnosis):
```
cd /projects/elohim/elohim/elohim-render && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/er-test RUSTC_WRAPPER="" cargo test --lib 2>&1 | tail -40
```
Final gate per touched crate (fmt/clippy):
```
cd <crate> && RUSTFLAGS="<per-crate>" CARGO_TARGET_DIR=/tmp/<x>-test RUSTC_WRAPPER="" cargo clippy -- -D warnings 2>&1 | tail -40
cd <crate> && cargo fmt --check
```
Rules (memory): `RUSTFLAGS=""` (doorway, elohim-compute, elohim-render — native); `RUSTFLAGS='--cfg getrandom_backend="custom"'` (elohim-storage — WASM); `RUSTC_WRAPPER=""` (sccache spawn-ENOENT); `/tmp` target dirs (fingerprint-ENOENT); **plain `cargo test`, NEVER nextest**; never `&&`-pipe a gate exit code — use `2>&1 | tail -N`.

---

## TASK 1 — `elohim_compute::backoff::jittered` (OWNS S7)

Files:
- **C** `elohim/elohim-compute/src/backoff.rs`
- **M** `elohim/elohim-compute/src/lib.rs` (append `pub mod backoff;` + `pub use backoff::jittered;`)
- **M** `elohim/elohim-compute/Cargo.toml` (add `rand`)

**Dep note:** `rand` is the canonical full-jitter source. Confirm it is not already present before adding (`grep -n '^rand' elohim/elohim-compute/Cargo.toml`); if `getrandom`/`rand` is already a transitive of `peers.rs`, prefer reusing it. `jittered` MUST be deterministic-testable: take the random factor via a small seam so tests assert bounds without flake — implement `jittered` as a thin wrapper over `jittered_with(base, max, attempt, frac: f64)` where `frac ∈ [0,1)` is the jitter fraction; `jittered` supplies `rand::random::<f64>()`. Tests call `jittered_with` with fixed fractions.

- [ ] Write the failing test — `elohim/elohim-compute/src/backoff.rs` (`#[cfg(test)] mod tests`):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn grows_exponentially_until_capped() {
        // frac=1.0 (max jitter) gives the full capped ceiling.
        assert_eq!(jittered_with(Duration::from_secs(1), Duration::from_secs(60), 0, 1.0), Duration::from_secs(1));
        assert_eq!(jittered_with(Duration::from_secs(1), Duration::from_secs(60), 1, 1.0), Duration::from_secs(2));
        assert_eq!(jittered_with(Duration::from_secs(1), Duration::from_secs(60), 3, 1.0), Duration::from_secs(8));
        // attempt 6 -> 64s, clamped to 60s ceiling.
        assert_eq!(jittered_with(Duration::from_secs(1), Duration::from_secs(60), 6, 1.0), Duration::from_secs(60));
    }

    #[test]
    fn full_jitter_zero_frac_is_zero() {
        assert_eq!(jittered_with(Duration::from_secs(5), Duration::from_secs(60), 3, 0.0), Duration::ZERO);
    }

    #[test]
    fn jitter_stays_within_capped_window() {
        // half jitter of an 8s capped window is 4s.
        assert_eq!(jittered_with(Duration::from_secs(1), Duration::from_secs(60), 3, 0.5), Duration::from_secs(4));
    }

    #[test]
    fn live_jittered_never_exceeds_cap() {
        for attempt in 0..12 {
            let d = jittered(Duration::from_millis(100), Duration::from_secs(30), attempt);
            assert!(d <= Duration::from_secs(30), "attempt {attempt} exceeded cap: {d:?}");
        }
    }

    #[test]
    fn attempt_overflow_saturates_at_cap() {
        // huge attempt must not panic on shift overflow; clamps to max.
        assert_eq!(jittered_with(Duration::from_secs(1), Duration::from_secs(60), 99, 1.0), Duration::from_secs(60));
    }
}
```
- [ ] Run, expect FAIL: `cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo test --lib backoff 2>&1 | tail -40` — expect `cannot find function jittered`.
- [ ] Write minimal implementation — `elohim/elohim-compute/src/backoff.rs`:
```rust
//! Shared reconnect/retry backoff primitive (Cat-C operational helper).
//!
//! Full-jitter exponential backoff (AWS "Exponential Backoff And Jitter").
//! The herd-resistant primitive both the storage conductor_client and the
//! doorway worker/conductor reconnect loops consume — one definition, not two
//! (the CircuitBreaker single-owner precedent).
use std::time::Duration;

/// Exponential `base * 2^attempt` clamped to `max`, then full-jitter
/// randomized in `[0, capped]`. Deterministic seam: `frac` is the jitter
/// fraction (`[0.0, 1.0]`); `jittered` supplies `rand::random()`.
pub fn jittered_with(base: Duration, max: Duration, attempt: u32, frac: f64) -> Duration {
    let base_ns = base.as_nanos();
    // Saturating shift: cap the exponent so 1u128 << attempt never overflows.
    let factor: u128 = 1u128.checked_shl(attempt.min(63)).unwrap_or(u128::MAX);
    let grown_ns = base_ns.saturating_mul(factor);
    let capped_ns = grown_ns.min(max.as_nanos());
    let jittered_ns = (capped_ns as f64 * frac.clamp(0.0, 1.0)) as u128;
    Duration::from_nanos(jittered_ns.min(u64::MAX as u128) as u64)
}

/// Full-jitter backoff using the thread RNG.
pub fn jittered(base: Duration, max: Duration, attempt: u32) -> Duration {
    jittered_with(base, max, attempt, rand::random::<f64>())
}
```
  Append to `lib.rs` (RESOLUTION-E append-only block, after line 13 `pub mod resources;`): `pub mod backoff;` and (in the `pub use` block after :21) `pub use backoff::jittered;`. Add `rand` to `Cargo.toml` `[dependencies]` if absent.
- [ ] Run, expect PASS (5 tests).
- [ ] Commit:
```
git add elohim/elohim-compute/src/backoff.rs elohim/elohim-compute/src/lib.rs elohim/elohim-compute/Cargo.toml
git commit -m "feat(elohim-compute): jittered() full-jitter backoff primitive (S7)

Single-owner herd-resistant backoff; deterministic via jittered_with(frac).
Consumed by storage conductor_client + doorway worker/conductor.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 2 — storage `conductor_client`: replace fixed 5s with growth + jitter (config)

Files:
- **M** `elohim/elohim-storage/src/conductor_client.rs:43-58` (config fields + default).

The current `reconnect_delay: Duration` (fixed 5s) is a single value re-used at three sleep sites (232/251/334) with no growth or jitter. Replace the SEMANTICS while keeping a backward-compatible field: keep `reconnect_delay` as the BASE, add `reconnect_max`, and track an attempt counter in the reconnect loop (Task 3). This task adds the config; Task 3 wires the loop.

- [ ] Write the failing test — append to `conductor_client.rs` `#[cfg(test)] mod tests` (or create one):
```rust
    #[test]
    fn reconnect_config_has_base_and_max() {
        let cfg = ConductorClientConfig::default();
        assert_eq!(cfg.reconnect_delay, STORAGE_RECONNECT_BASE);
        assert_eq!(cfg.reconnect_max, STORAGE_RECONNECT_MAX);
        assert!(cfg.reconnect_max > cfg.reconnect_delay, "max must exceed base");
    }
```
- [ ] Run, expect FAIL: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo test --lib conductor_client 2>&1 | tail -40` — expect `no field reconnect_max` / `cannot find value STORAGE_RECONNECT_BASE`.
- [ ] Write minimal implementation — add consts above `ConductorClientConfig` and a `reconnect_max` field:
```rust
/// Base reconnect delay for the conductor client. Grows exponentially with
/// full jitter (elohim_compute::backoff::jittered) up to STORAGE_RECONNECT_MAX
/// so N storage nodes do not reconnect in lockstep after a conductor blip.
const STORAGE_RECONNECT_BASE: Duration = Duration::from_secs(1);
/// Ceiling for the jittered reconnect delay.
const STORAGE_RECONNECT_MAX: Duration = Duration::from_secs(60);
```
  In the struct: add `pub reconnect_max: Duration,` after `reconnect_delay`. In `Default`: set `reconnect_delay: STORAGE_RECONNECT_BASE,` (was `from_secs(5)`) and add `reconnect_max: STORAGE_RECONNECT_MAX,`.
- [ ] Run, expect PASS.
- [ ] Commit:
```
git add elohim/elohim-storage/src/conductor_client.rs
git commit -m "feat(elohim-storage): conductor reconnect base+max config (jitter prep)

Replaces fixed 5s reconnect_delay with a base (1s) + max (60s) for
jittered exponential backoff. Loop wiring in follow-up.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 3 — storage `conductor_client`: wire jittered backoff into the reconnect loop

Files:
- **M** `elohim/elohim-storage/src/conductor_client.rs` (the reconnect loop around lines 225-340; import `elohim_compute::backoff::jittered`).

Replace the three fixed `tokio::time::sleep(config.reconnect_delay)` sites with a jittered delay driven by an attempt counter local to the reconnect loop. On a successful (sustained) connection, reset the attempt counter to 0; on each failed connect, increment. Use `jittered(config.reconnect_delay, config.reconnect_max, attempt)`.

- [ ] **First verify the loop structure** (`Read` lines 220-345) — the three sleep sites may share one loop scope. The attempt counter must be declared OUTSIDE the retry loop and reset on a healthy session (mirror doorway's `STABLE_SESSION_THRESHOLD` discipline if a session-length signal exists; if not, reset on the first successful request post-connect). Document the chosen reset point in the commit.
- [ ] Write the failing test — a pure helper test asserting the loop's delay-selection logic (the loop itself needs a live conductor; test the decision):
```rust
    #[test]
    fn reconnect_delay_grows_with_attempt_and_caps() {
        use elohim_compute::backoff::jittered;
        let cfg = ConductorClientConfig::default();
        for attempt in 0..10 {
            let d = jittered(cfg.reconnect_delay, cfg.reconnect_max, attempt);
            assert!(d <= cfg.reconnect_max, "attempt {attempt} exceeded max");
        }
    }
```
- [ ] Run, expect FAIL (compile: `jittered` unused/import) then implement.
- [ ] Write minimal implementation — declare `let mut reconnect_attempt: u32 = 0;` before the reconnect loop; replace each `tokio::time::sleep(config.reconnect_delay).await;` with:
```rust
            let delay = elohim_compute::backoff::jittered(
                config.reconnect_delay,
                config.reconnect_max,
                reconnect_attempt,
            );
            reconnect_attempt = reconnect_attempt.saturating_add(1);
            tokio::time::sleep(delay).await;
```
  Reset `reconnect_attempt = 0;` at the post-connect success point. **Verify `elohim_compute` is already a dep of elohim-storage** (`grep elohim-compute elohim/elohim-storage/Cargo.toml`); if absent, add it (path dep) — note in commit.
- [ ] Run, expect PASS + clippy clean.
- [ ] Commit:
```
git add elohim/elohim-storage/src/conductor_client.rs
git commit -m "fix(elohim-storage): jittered exponential conductor reconnect (anti-herd)

Fixed 5s lockstep sleep -> jittered(base,max,attempt) via
elohim_compute::backoff. Attempt resets on a healthy session.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 4 — doorway `ReconnectBackoff`: add jitter to existing exponential ladder

Files:
- **M** `doorway/doorway-service/src/worker/conductor.rs:56-82` (the `ReconnectBackoff` struct + `next_after_connect_failure`).

The ladder already grows + resets-on-stable-session. The ONLY gap is jitter on the returned delay. Minimal change: track an `attempt: u32` and return `jittered(BASE_RECONNECT_DELAY, MAX_RECONNECT_DELAY, attempt)` instead of the raw doubled value — preserving the stable-session reset (`next_after_session`) and the cap. This keeps the existing storm tests (`backoff_caps_at_max`, `backoff_escalates_on_unstable_sessions`, `backoff_resets_after_stable_session`) green — **they assert ordering/caps, which jitter must NOT break**: jitter is `[0, capped]`, so a jittered escalation is `<= capped` and the cap test still holds; re-read those tests before changing and adjust assertions ONLY if they assert exact equality (use the `jittered_with(..,1.0)` upper-bound semantics or relax to `<=`).

- [ ] Write the failing test — append to `worker/conductor.rs` `#[cfg(test)] mod tests`:
```rust
    #[test]
    fn backoff_applies_jitter_within_cap() {
        let mut b = ReconnectBackoff::new();
        for _ in 0..20 {
            let d = b.next_after_connect_failure();
            assert!(d <= MAX_RECONNECT_DELAY, "jittered delay exceeded cap: {d:?}");
        }
    }
```
- [ ] Run, expect FAIL (if `next_after_connect_failure` currently returns un-jittered exact doublings the new bound holds trivially — make the test discriminating by asserting that across many calls at the SAME attempt the values VARY, proving jitter is live):
```rust
    #[test]
    fn backoff_jitter_varies_across_runs() {
        let sample = |attempt: u32| {
            let mut b = ReconnectBackoff::new();
            for _ in 0..attempt { let _ = b.next_after_connect_failure(); }
            b.next_after_connect_failure()
        };
        let a = sample(6);
        let b = sample(6);
        let c = sample(6);
        assert!(!(a == b && b == c), "expected jitter variance at attempt 6, got identical {a:?}");
    }
```
- [ ] Write minimal implementation — add `attempt: u32` to `ReconnectBackoff`; in `next_after_connect_failure` compute `let d = elohim_compute::backoff::jittered(BASE_RECONNECT_DELAY, MAX_RECONNECT_DELAY, self.attempt); self.attempt = self.attempt.saturating_add(1); d`; in `next_after_session` reset `self.attempt = 0` (alongside the existing `self.delay = BASE_RECONNECT_DELAY`) when stable, then call `next_after_connect_failure`. Keep `delay` field if other code reads it, else remove. **Verify `elohim_compute` is a doorway dep** (it is — `peers.rs` is consumed; confirm `grep elohim-compute doorway/doorway-service/Cargo.toml`).
- [ ] Run, expect PASS (new + existing backoff tests). If an exact-equality existing assertion now flakes, relax it to `<=` per the note above — document in commit.
- [ ] Commit:
```
git add doorway/doorway-service/src/worker/conductor.rs
git commit -m "fix(doorway): add full jitter to conductor ReconnectBackoff (anti-herd)

Exponential ladder + stable-session reset preserved; jitter via
elohim_compute::backoff::jittered breaks lockstep reconnect of N pods.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 5 — delete dead `rate_limit_rpm` config theater (RESOLUTION-I)

Files:
- **M** `doorway/doorway-service/src/services/route_registry.rs` (field at :91 + every constructor that sets it: :226,:475,:495,:516,:550,:574,:593,:612,:631,:650,:712,:730,:1062,:1076)
- **M** `doorway/doorway-service/src/server/http.rs:4183,4260` (two `rate_limit_rpm: 0` literals feeding the registry struct)

Pure deletion — `grep` confirmed ZERO consumers gate on it (`grep -rn "rate_limit_rpm" doorway/doorway-service/src` shows only the struct field + its setters, never a reader that enforces). The real defenses are Pillar-2 admission shed + `UpstreamBreakers`.

- [ ] Write the failing test (compile-as-test) — there is no behavior to assert; the "test" is that the crate compiles with the field GONE. First confirm no reader: `grep -rn "\.rate_limit_rpm\b" doorway/doorway-service/src | grep -v ': *rate_limit_rpm'` MUST return nothing (no `.rate_limit_rpm` field READ). Capture this grep output in the commit body as the evidence.
- [ ] Run the grep, expect EMPTY (no readers).
- [ ] Implement — remove `pub rate_limit_rpm: u32,` from the struct at :91 and delete every `rate_limit_rpm: <literal>,` initializer line (registry + http.rs). Use `Edit` per occurrence (the lines are not identical contexts; do NOT `replace_all` blindly — each sits in a different struct literal).
- [ ] Run, expect PASS: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib route_registry 2>&1 | tail -40` (compiles + existing registry tests green).
- [ ] Commit:
```
git add doorway/doorway-service/src/services/route_registry.rs doorway/doorway-service/src/server/http.rs
git commit -m "refactor(doorway): delete dead rate_limit_rpm config theater

Threaded but never enforced (zero gating consumers). Real defenses are
Pillar-2 admission shed + UpstreamBreakers. Removing avoids implying a
limiter that does not exist.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 6 — migrate lower-traffic per-request `Client::new()` routes to the shared pooled client

Files:
- **M** `doorway/doorway-service/src/routes/{threshold.rs:45, elohim_agent.rs:87, epr.rs:40, collectives.rs:144, apps.rs:224/373/572, seed.rs:278, blob.rs:493}`

Each builds a fresh, UNBOUNDED, connection-pool-less `reqwest::Client::new()` per request — the same wedge class as the (already-fixed) hot path, at lower traffic. Repoint each to `state.storage_proxy_client` (the pooled, timeout-bounded `Arc<reqwest::Client>` already on `AppState`).

**Per-route verification gate (do this FIRST per file):** confirm the handler has `state: &AppState` (or `&Arc<AppState>`) in scope. If a handler does NOT receive `AppState` (some routes take only the request), do NOT thread state through a signature change in this plan — instead introduce a process-wide `OnceLock<reqwest::Client>` pooled client local to that route module and document it as a follow-on candidate for AppState consolidation. Prefer `state.storage_proxy_client` wherever state is in scope.

Sub-tasks (one commit per file, TDD where a test exists; otherwise compile + the route's existing tests):
- [ ] For each file: `Read` the handler signature + the `Client::new()` line.
- [ ] Where `state.storage_proxy_client` is reachable: replace `let client = reqwest::Client::new();` with `let client = state.storage_proxy_client.clone();` (or borrow `&state.storage_proxy_client` if `.get/.post` chains accept `&`). Note `apps.rs:572` is an inline `reqwest::Client::new().head(...)` — repoint to the shared client too. `blob.rs:1006` is `#[cfg(test)]` (the proxy-addr test) — LEAVE it.
- [ ] Where state is NOT reachable: add a module `static SHARED: OnceLock<reqwest::Client>` built with the same timeout bounds as `init_storage_proxy_client()` (DRY note: ideally call the same builder; if it's private to `server/http.rs`, this plan exposes a `pub(crate) fn pooled_client_config()` returning the `ClientBuilder` — SOLE owner of that small extraction, flag in hand-off).
- [ ] Run per touched file: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib <module> 2>&1 | tail -30`.
- [ ] Commit per file (or a single commit listing all files if they share no test):
```
git add doorway/doorway-service/src/routes/<file>.rs
git commit -m "perf(doorway): route <file> uses pooled storage_proxy_client (residual wedge)

Replaces per-request unbounded reqwest::Client::new() with the shared
connect+request-timeout-bounded pool. Same wedge class as the fixed hot path.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 7 — DIAGNOSE the render wedge (instrument `sync_channel(1)`, NO behavior change)

Files:
- **M** `elohim/elohim-render/src/angular.rs` (instrument `:112` `mpsc::sync_channel::<StringWorkItem>(1)` saturation — additive only)

This is the **highest-leverage** item (the long pole) and a DIAGNOSE task, not a fix. Hypothesis: render concurrency is gated by isolate count, not by `render_semaphore` (default 8) — the semaphore admits 8 SSR requests but they serialize on a single-slot per-isolate channel, so under sustained load on a `cpu:1` cgroup renders queue head-of-line behind one in-flight isolate → `/health` starvation → settled-pod wedge with zero churn.

- [ ] Write the failing test — add a unit test in `angular.rs` asserting the **observable instrumentation exists** (a counter/gauge the diagnosis surfaces), e.g. a `pub fn pending_render_slots()` or a `tracing` span with `queue_depth`:
```rust
    #[test]
    fn render_queue_exposes_saturation_signal() {
        // The instrumentation must expose how full the per-isolate sync_channel is.
        // (Asserts the diagnostic accessor exists; value semantics tested by the
        // saturation behavior test below.)
        let r = AngularRenderer::new_for_test(); // or the existing test constructor
        assert_eq!(r.queue_depth_hint(), 0, "fresh renderer has empty queue");
    }
```
  (If `AngularRenderer` has no test constructor, instead assert via a free helper that the `try_send`-full branch increments a `static AtomicU64 RENDER_QUEUE_FULL_TOTAL` — testable without V8.)
- [ ] Run, expect FAIL (`no method queue_depth_hint` / `cannot find RENDER_QUEUE_FULL_TOTAL`).
- [ ] Implement (instrumentation ONLY — do NOT change the channel bound from 1 in this task; that is the fix, deferred to a follow-on once diagnosis confirms): at the send site (`:244` `let work_item = StringWorkItem {...}`), switch the blocking `send` to a `try_send` with a fallback to blocking `send` on full, incrementing a `static RENDER_QUEUE_FULL_TOTAL: AtomicU64` and emitting `tracing::warn!(target: "ssr.render", "render isolate channel full — head-of-line serialization")`. Expose `RENDER_QUEUE_FULL_TOTAL.load(...)` via a `pub fn render_queue_full_total() -> u64` so the diagnosis surfaces through the existing render-stats / `x-ssr-*` path (SOFT dep on P-DIAGNOSTIC's surface — do NOT add a new endpoint; emit the counter and let the diagnostic track plumb it).
- [ ] Run, expect PASS.
- [ ] **DIAGNOSIS DELIVERABLE (in the commit body, not a separate .md):** record (a) whether `worker_threads()` (main.rs:53) actually returns >1 under a `cpu:1` cgroup — check the function and note the value it derives at cpu=1; (b) the relationship `render_semaphore(8)` vs `sync_channel(1)` — confirm the serialization claim by reading the rx loop at `:137`; (c) the RECOMMENDED FIX for the follow-on (raise the channel bound to match isolate pool size, OR pool N isolates, OR move the semaphore to gate isolates). State explicitly: this task only INSTRUMENTS; the fix is a named follow-on so the diagnosis can be confirmed against a real load trace first (systematic-debugging discipline: do not blind-fix).
- [ ] Commit:
```
git add elohim/elohim-render/src/angular.rs
git commit -m "diag(elohim-render): instrument per-isolate sync_channel(1) saturation

Counts head-of-line stalls on the single-slot render queue + warn! to
surface the suspected settled-pod wedge mechanism (zero-churn, ~2.5min).
Diagnosis only; channel-bound fix deferred pending a confirming load trace.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 8 — periodic-conductor-call inventory guard (regression prevention)

Files:
- **M** `doorway/doorway-service/src/services/federation.rs` (around the `record_health_attestation` call at :289 — add a guard/test, not behavior)

`record_health_attestation` (every ~5th 60s tick) is the next `record_heartbeat`-class periodic conductor call. It now correctly relies on iteration-3 (`is_transport_error`) keeping the connection on an app-level Err. The risk: if that fn is retired/skewed like `record_heartbeat` was (federation.rs:208 documents that retirement), the connection-management invariant silently regresses. Add a lightweight guard test documenting the invariant.

- [ ] Write the test — a unit test that asserts the periodic-call set is KNOWN (a `const PERIODIC_CONDUCTOR_FNS: &[&str] = &["record_health_attestation"];` inventory) and that any addition is deliberate:
```rust
    #[test]
    fn periodic_conductor_call_inventory_is_explicit() {
        // Regression guard: every periodic (timer-driven) conductor coordinator
        // call MUST be listed here so reviewers notice a new record_heartbeat-class
        // driver. Adding a periodic call without updating this list fails review.
        assert_eq!(PERIODIC_CONDUCTOR_FNS, &["record_health_attestation"]);
    }
```
- [ ] Run, expect FAIL (`cannot find PERIODIC_CONDUCTOR_FNS`).
- [ ] Implement — add the const near the call site with a doc-comment naming the iteration-3 dependency:
```rust
/// Inventory of timer-driven (periodic) conductor coordinator calls. Each is a
/// potential record_heartbeat-class connection driver and relies on
/// zome_caller::is_transport_error keeping the connection on an app-level Err.
/// Adding one without updating this list (and the guard test) is a review smell.
const PERIODIC_CONDUCTOR_FNS: &[&str] = &["record_health_attestation"];
```
- [ ] Run, expect PASS.
- [ ] Commit:
```
git add doorway/doorway-service/src/services/federation.rs
git commit -m "test(doorway): periodic conductor-call inventory guard

Documents record_health_attestation as the live record_heartbeat-class
periodic driver; guard test flags any new periodic conductor call for review.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## FINAL GATE (run after all tasks; per-crate)
```
# elohim-compute
cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo test --lib 2>&1 | tail -40
cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo clippy -- -D warnings 2>&1 | tail -40
cd /projects/elohim/elohim/elohim-compute && cargo fmt --check
# elohim-storage (WASM flag)
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo test --lib conductor_client 2>&1 | tail -40
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo clippy -- -D warnings 2>&1 | tail -40
cd /projects/elohim/elohim/elohim-storage && cargo fmt --check
# doorway (native)
cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib --bins 2>&1 | tail -40
cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo clippy -- -D warnings 2>&1 | tail -40
cd /projects/elohim/doorway/doorway-service && cargo fmt --check
# elohim-render (native)
cd /projects/elohim/elohim/elohim-render && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/er-test RUSTC_WRAPPER="" cargo test --lib 2>&1 | tail -40
cd /projects/elohim/elohim/elohim-render && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/er-test RUSTC_WRAPPER="" cargo clippy -- -D warnings 2>&1 | tail -40
cd /projects/elohim/elohim/elohim-render && cargo fmt --check
```

---

## // FOLLOW-ON seams (deliberately left for the integration pass)

- **feat→dev integration of iteration-3** (`89bc208a8` `zome_caller.rs is_transport_error`). Built on feat, ABSENT on dev (verified). This is an INTEGRATION action (the integrator's feat→dev merge), not new code. Per the review §6 it likely does NOT cure the zero-churn render wedge, so do NOT gate T7's diagnosis on it.
- **The render-channel FIX itself** (raise `sync_channel(1)` bound / pool N isolates / gate the semaphore on isolates). T7 only INSTRUMENTS. The fix lands after the diagnosis is confirmed against a real load trace (systematic-debugging: no blind fix). Named follow-on plan.
- **`pooled_client_config()` extraction** (if T6 needs the `init_storage_proxy_client` builder shared with route modules that lack `AppState`): exposing a `pub(crate) fn` in `server/http.rs` is a small extraction this plan flags but the integrator should confirm doesn't collide with any AppState refactor.
- **`rate_limit_rpm` as an actuatable knob (NOT this plan).** Per the open decision, the recommendation is DELETE (T5). If the operator instead wants a real per-route limiter, it must become an instance of P-ACTUATION's `Actuation` contract (S2), not a bespoke counter — a P-ACTUATION follow-on, never a P-DEFENSE bespoke limiter.
- **Storage-side `BackoffLadder` stateful wrapper** — Task 1 sketches it but only ships `jittered`. If P-RECONCILE wants a stateful ladder for cadence retries, promote `BackoffLadder` to a re-export then (additive, RESOLUTION-E).
- **`x-ssr-*` / render-stats plumbing of `render_queue_full_total()`** is a SOFT hand-off to P-DIAGNOSTIC — T7 emits the counter; the diagnostic track surfaces it. Do not add an endpoint here.

---

## Dispatch note

- **Isolated worktree, subagent-driven, commit-only.** Run in an isolated worktree (or selective-stage exactly the files in each task's `git add`). Commit on the shift branch only; **the integrator pushes** — never `git push` from this plan.
- **Wave 1 root:** dispatch in parallel with P-ACTUATION / P-RECONCILE / P-PROOFS-core. Zero file-sequencing against other plans. The only cross-plan seam is the append-only `elohim-compute/src/lib.rs` re-export block (RESOLUTION-E) and `elohim-compute/Cargo.toml` (P-DEFENSE-exclusive) — both mechanical merges.
- **Task order within this plan:** T1 (owns S7) → T2 → T3 (T3 hard-depends on T1+T2) ; T4 hard-depends on T1; T5, T6, T7, T8 are independent and may run in any order / parallel subagents. Recommended: T1 first (unblocks T3/T4), then fan out.
- **No `.claude/data` writes from runtime Rust** (the elevate arm is an external poller) — none of these tasks write there.
