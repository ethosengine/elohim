# Sweep Registry & Cadence Reconcile — Implementation Plan (P-RECONCILE)

> For agentic workers: REQUIRED SUB-SKILL: superpowers:subagent-driven-development (or executing-plans). Steps use checkbox (- [ ]) syntax. TDD: every task writes the failing test BEFORE the implementation.
> Working draft — NOT cite-sealed. Authored against the P2P-Dataplane Contract Ledger (`/projects/elohim/P2P-DATAPLANE-CONTRACT-LEDGER-2026-06-14.md`) and Integration Plan (`/projects/elohim/P2P-DATAPLANE-INTEGRATION-2026-06-14.md`).

## 1. Context / Why — closes finding #4 (sync/reconcile scatter)

**Goal:** Give `elohim-storage`'s P2P node a single, observable, actuatable home for its periodic background sweeps. Today the `P2PNode::run()` select-loop hand-rolls **ELEVEN** `tokio::time::interval` arms (status, sync, replication, gap-dispatch, acquisition-reconcile, provide-reconcile, acquisition-dispatch, drain, bootstrap-retry, inventory-broadcast, custody-sweep — `mod.rs:2202–2307`). Each arm:
- resolves its cadence **once at `run()` entry** and can never be reconfigured live (the `TODO(T22-live-reconfig)` at `mod.rs:2249–2255` is the canonical scar — `P2PCommand::ReconfigureCadence` is named there as the missing primitive);
- exposes **no per-sweep observability** — an operator cannot see when a sweep last ran, whether it is enabled, or how many ticks it skipped under backpressure (the `if self.sync_paused { skip }` arms silently drop ticks with no counter);
- cannot be paused/resumed/run-now individually for incident response.

This is **finding #4: sync/reconcile cadence scatter** — eleven ad-hoc timers with no registry, no read-model, no lifecycle surface. This plan introduces a descriptor-only **`Sweep` trait** (S5), a per-sweep **`SweepStatus`** read-model + **`SweepRegistrySnapshot`** (S5), a storage-local **`SweepRegistry`** (S6) that owns each sweep's tunable interval + observability counters, three **`P2PCommand` lifecycle variants** (ReconfigureCadence / SetSweepEnabled / RunSweepNow), and the **structural `run()` refactor** (S9) that routes the timer arms through the registry. The cadence read-model embeds into `P2PStatusInfo` so the diagnostic track can surface it.

**What this is NOT.** This is NOT a rewrite of any sweep's *body* — the async work (`run_replication_cycle`, `drain_publish_queue`, `run_custody_reconcile`, etc.) stays verbatim on `P2PNode` (it closes over the non-Send swarm). The registry owns only the **schedule + observability + lifecycle**, never the work. It does NOT fold timers into `ReconcileController::run_loop` (that is signal-driven, a different shape — ledger §5 DO-NOT: "do NOT fold timers into it").

---

## 2. OWNED FILES (verbatim from ledger §2 + the 4-mutator audit finding)

**CREATE (C) — SOLE owner:**
- `elohim/elohim-compute/src/sweep.rs` — `Sweep` trait + `SweepStatus` + `SweepRegistrySnapshot` (S5). SOLE owner (ledger §2 P-RECONCILE).
- `elohim/elohim-storage/src/p2p/sweep_registry.rs` — `SweepRegistry` (S6), storage-local (closes over non-Send swarm scheduling state). SOLE owner.

**MUTATE (M):**
- `elohim/elohim-storage/src/p2p/mod.rs` — **PRIMARY structural owner.** Add 3 `P2PCommand` variants (ReconfigureCadence / SetSweepEnabled / RunSweepNow + their `handle_command` arms + the in-test no-op arms at `:1099` region); refactor the `run()` select-loop (`:2197–2475`) to drive its periodic arms through the `SweepRegistry`; embed `SweepRegistrySnapshot` into `P2PStatusInfo` (`:707`) + populate in `refresh_status()` (`:7050`); add the `sweep_registry` field to the `P2PNode` struct (`:470` region) + both constructors (`:1152`/`:1692` paths) + every `P2PStatusInfo { .. }` literal (`:1152`, `:1692`, `:7050`).
- `elohim/elohim-compute/src/lib.rs` — append-only re-export block for `sweep` (RESOLUTION-E; one `pub mod sweep;` + one `pub use sweep::{...};`).
- `elohim/elohim-storage/src/config.rs` — append-only additive cadence fields seeding the registry (`#[serde(default = ...)]` + matching `Default` lines; RESOLUTION-C — storage `Config`).

> **PRIMARY-OWNER-OF-mod.rs STATEMENT (ledger RESOLUTION-B + integration G0-B).** `p2p/mod.rs` has **FOUR** mutators in this campaign: **(1) P-RECONCILE — me — the PRIMARY structural owner** (the `run()` select-loop rewrite + the `P2PCommand` enum surgery + the `SweepRegistrySnapshot` embed — the deep change); **(2) P-TRANSPORT** (one `connection_limits` event-arm match + the 3 additive `P2PConfig` limit fields at `mod.rs:374` + `Default`); **(3) P-DIAGNOSTIC** (2 additive `bool` fields on `P2PStatusInfo` + populate); **(4)** the snapshot-embed is mine. **All three non-primary touches SEQUENCE BEHIND my structural change** — they rebase onto the rewritten `run()` and the extended `P2PStatusInfo` *after* I land and merge. They make additive, append-only edits that slot into my refactored select-loop / struct; I make no accommodation for them beyond leaving the named FOLLOW-ON seams (§7). I land FIRST in Wave 1.

> **NOTE — storage `P2PConfig` lives in `p2p/mod.rs:374`, NOT `config.rs`** (integration G0-B / P-TRANSPORT SEAM-DELTA). My cadence config fields go on the storage **`Config`** in `config.rs` (which `P2PNode` reads via `self.config`), consistent with the existing `inventory_broadcast_seconds` / `custody_sweep_seconds` fields already there (`config.rs:141,145`). RESOLUTION-C (3-way additive merge) applies to my `config.rs` fields; the *node's* `steward/node/src/config.rs` is a different file I do not touch.

**NEVER-TOUCH (explicit non-goals — read-only guards; altering any is out of scope and forbidden):**
- **GapTracker never-immediate-requeue** — the replication gap-queue's anti-thrash invariant (`gap_queue` / `drain_gap_queue`). I schedule the *tick*; I never touch requeue policy.
- **DispatchBudget credit accounting** — `MAX_REPLICATION_INFLIGHT` per-tick budgeting inside `drain_gap_queue` / `drain_acquisition_queue`. The registry caps *cadence*, never *in-flight credits*.
- **`in_scope_of` upsert guard** — the provide/acquisition reconcile scope filter. Untouched.
- **`DedupLru` / `WriteThroughState` poison policies** — `dedup.insert()` (`mod.rs:5593`) and the `.unwrap_or_else(|poison| poison.into_inner())` recovery in `write_through.rs`. Untouched; `WriteThroughState`'s 4-layer override SHAPE is mirrored by P-ACTUATION, not me.
- **`sync_paused` backpressure semantics** — I preserve every existing `if self.sync_paused { skip }` gate verbatim inside the refactored arms; the registry counts the skip (`skips_total`) but does not change WHETHER a paused arm skips.

---

## 3. NEW PRIMITIVES I OWN (signatures pinned to ledger §3) + CONSUMED primitives

### S5 — `elohim_compute::sweep` (CREATE `elohim/elohim-compute/src/sweep.rs`)

Descriptor-only trait (NO async body — bodies stay on `P2PNode`), plus the observable read-model. All `Serialize` + `TS` for the diagnostic embed.

```rust
use std::time::Duration;

/// Descriptor for a periodic background sweep. The IMPLEMENTOR carries only
/// identity + default cadence — the async work stays on the owning node
/// (it closes over non-Send runtime state). Used as a static registration
/// descriptor for the SweepRegistry.
pub trait Sweep {
    /// Stable, unique sweep name (registry key + wire identifier).
    fn name(&self) -> &'static str;
    /// Archetype/config-independent default tick interval.
    fn default_cadence(&self) -> Duration;
}

/// Per-sweep observable read-model. One per registered sweep.
#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct SweepStatus {
    pub name: String,
    pub enabled: bool,
    #[ts(type = "number")]
    pub cadence_secs: u64,
    /// Wall-clock ms of the last completed run; None until first run.
    #[ts(type = "number | null")]
    pub last_run_ms: Option<u64>,
    /// Wall-clock ms the next tick is expected; None when disabled.
    #[ts(type = "number | null")]
    pub next_tick_ms: Option<u64>,
    /// True while a tick body is executing (re-entrancy / overrun signal).
    pub in_flight: bool,
    #[ts(type = "number")]
    pub runs_total: u64,
    /// Ticks dropped (backpressure skip / disabled / still in-flight).
    #[ts(type = "number")]
    pub skips_total: u64,
    pub last_error: Option<String>,
}

/// Snapshot of the whole registry — embedded in P2PStatusInfo (S9).
#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct SweepRegistrySnapshot {
    pub sweeps: Vec<SweepStatus>,
}
```

> Ledger §3 canonical rows pin these field sets verbatim (`SweepStatus` = `{name,enabled,cadence_secs,last_run_ms,next_tick_ms,in_flight,runs_total,skips_total,last_error}`; `SweepRegistrySnapshot` = `{sweeps: Vec<SweepStatus>}`). I MUST NOT rename or add fields outside this set — P-DIAGNOSTIC embeds the snapshot as-is.

### S6 — `SweepRegistry` (CREATE `elohim/elohim-storage/src/p2p/sweep_registry.rs`)

Storage-local (it holds the per-sweep `tokio::time::Interval` scheduling state + `DashMap` of mutable status, which close over the run-loop). Ledger §3 row: `DashMap<&'static str, SweepHandle> + pause/resume/run_now/reconfigure(name[,secs]) -> Result<(),ActuationRefusal>`.

```rust
use dashmap::DashMap;
use elohim_compute::actuation::ActuationRefusal; // CONSUMED S1 — skip-if-present (see below)
use elohim_compute::sweep::{SweepRegistrySnapshot, SweepStatus};

/// Per-sweep mutable scheduling + observability state. Held in the registry;
/// the run-loop reads `due()`/records run outcomes. The `Interval` is NOT here
/// (it cannot be Send-shared across the select arms); the registry stores the
/// cadence as Duration and the run-loop owns the Interval, re-armed on
/// reconfigure via a `dirty` flag the loop polls.
pub struct SweepHandle {
    pub name: &'static str,
    pub enabled: AtomicBool,
    pub cadence_secs: AtomicU64,
    /// Set true by reconfigure(); run-loop clears it after re-arming its Interval.
    pub cadence_dirty: AtomicBool,
    /// Set true by run_now(); run-loop fires the body once then clears.
    pub run_now: AtomicBool,
    pub last_run_ms: AtomicU64,   // 0 = never
    pub in_flight: AtomicBool,
    pub runs_total: AtomicU64,
    pub skips_total: AtomicU64,
    pub last_error: Mutex<Option<String>>,
}

pub struct SweepRegistry {
    handles: DashMap<&'static str, Arc<SweepHandle>>,
}

impl SweepRegistry {
    pub fn new() -> Self { /* empty */ }

    /// Register a sweep from its descriptor + the effective (config-resolved)
    /// cadence. `enabled=false` registers a disabled sweep (e.g. inventory
    /// broadcast on the mobile archetype). Idempotent on name.
    pub fn register(&self, name: &'static str, cadence_secs: u64, enabled: bool) -> Arc<SweepHandle>;

    /// Read-model snapshot (Cat-C node-local read; ledger §6).
    pub fn snapshot(&self, now_ms: u64) -> SweepRegistrySnapshot;

    // ---- lifecycle (driven by the 3 P2PCommand variants) ----
    pub fn reconfigure(&self, name: &str, secs: u64) -> Result<(), ActuationRefusal>;
    pub fn set_enabled(&self, name: &str, enabled: bool) -> Result<(), ActuationRefusal>;
    pub fn run_now(&self, name: &str) -> Result<(), ActuationRefusal>;

    // ---- run-loop side (called from the refactored select arms) ----
    /// Record a tick that executed: bump runs_total, set last_run_ms, clear in_flight.
    pub fn record_run(&self, name: &str, now_ms: u64);
    /// Record a dropped tick (backpressure/disabled/in-flight): bump skips_total.
    pub fn record_skip(&self, name: &str);
    /// True if the sweep is enabled AND (timer due OR run_now latch set). The
    /// run-loop calls this in each arm before doing work; clears the run_now latch.
    pub fn should_fire(&self, name: &str) -> bool;
    /// Take-and-clear the cadence_dirty flag + return the new cadence so the
    /// run-loop re-arms its Interval. None when not dirty.
    pub fn take_cadence_change(&self, name: &str) -> Option<u64>;
}
```

**Refusal semantics (consumes S1):** `reconfigure`/`set_enabled`/`run_now` return `Err(ActuationRefusal { code: RefusalCode::NotActuatable, .. })` for an unknown sweep name, and `Err(.. RefusalCode::OutOfGrantBounds ..)` for a cadence below a hard floor (e.g. `secs < 1` — never let an operator set a 0s busy-loop; 0 means "disable", routed through `set_enabled`). The floor is a sweep-local rail, NOT the REA grant path (that is P-ACTUATION's Cat-A surface — see §6).

### The 3 `P2PCommand` variants (MUTATE `p2p/mod.rs`) — pinned to ledger §3

```rust
/// Reconfigure a sweep's tick cadence live. Closes the TODO(T22-live-reconfig)
/// at run() entry. `secs` must be >= 1 (0 → use SetSweepEnabled to disable).
ReconfigureCadence {
    sweep: String,
    secs: u64,
    reply: oneshot::Sender<Result<(), ActuationRefusal>>,
},
/// Pause (enabled=false) or resume (enabled=true) a sweep. A paused sweep's
/// select arm records a skip and does no work.
SetSweepEnabled {
    sweep: String,
    enabled: bool,
    reply: oneshot::Sender<Result<(), ActuationRefusal>>,
},
/// Fire a sweep's body once on the next loop iteration regardless of its timer.
RunSweepNow {
    sweep: String,
    reply: oneshot::Sender<Result<(), ActuationRefusal>>,
},
```

Their `handle_command` arms call the matching `self.sweep_registry.{reconfigure,set_enabled,run_now}(..)` and `let _ = reply.send(result)`. The in-test command-drain stub (`mod.rs:1099` region, the `match cmd` in the test `tokio::spawn`) gets three no-op arms `cmd @ (ReconfigureCadence|SetSweepEnabled|RunSweepNow) { reply, .. } => { let _ = reply.send(Ok(())); }` so the test handle keeps compiling.

### CONSUMED primitives (skip-if-present clause)

- **S1 `ActuationRefusal` / `RefusalCode`** (owner P-ACTUATION, `elohim_compute::actuation`) — `DEPENDS-ON: SOFT`. **Skip-if-present (verbatim ledger §1):** before landing, verify `elohim_compute::actuation` exposes `ActuationRefusal`/`RefusalCode`. If present, VERIFY-ONLY (import + use). If absent at my integration point (P-ACTUATION's Wave-1 `lib.rs` re-export slips), land a **temporary local shim** in `sweep_registry.rs` (`enum LocalRefusal` mirroring `{code, elevate}` + `enum RefusalCode { OutOfGrantBounds, GrantExpired, NotActuatable, GateRefused(String) }` verbatim from ledger §3), flag it in §7 hand-off notes, and **delete the shim at integration** (integration §2.2 S1 check greps for exactly one `enum RefusalCode` — mine must be gone). My command-reply type is `Result<(), ActuationRefusal>` — I do NOT invent a parallel `ReconfigureError` (ledger C2 forbids it).
- **`ScopeId` (S3, P-ACTUATION)** — OPTIONAL. The sweep cadence knobs MAY map to `ScopeId::SweepCadence(...)` strings (ledger §3 names this variant) for the actuation spine. Skip-if-present: if `elohim_compute::scope_vocab::ScopeId` exposes `SweepCadence`, use `scope_str()` for the registry key↔scope mapping in §7's actuation seam; if absent, the registry keys on `&'static str` sweep names directly (no scope vocab dependency for v1 — the literal sweep names ARE the keys). I do NOT define `ScopeId`.

---

## 4. DEPENDENCY EDGES (ledger §4 DAG — I am a WAVE-1 ROOT)

I have **ZERO inbound HARD edges** and one outbound SOFT edge. Roots (no outbound hard edge): P-ACTUATION, **P-RECONCILE**, P-DEFENSE, P-PROOFS.

| Edge | Type | Reason |
|------|------|--------|
| **P-RECONCILE → P-ACTUATION** | **SOFT** | My command replies return `ActuationRefusal` (S1). Works standalone with a local refusal shim if P-ACTUATION slips; delete shim on integration. |
| **P-DIAGNOSTIC → P-RECONCILE** | **HARD (I am the dependency)** | P-DIAGNOSTIC embeds my `SweepRegistrySnapshot` (S5) into `P2PStatusInfo` AND its 2-field mod.rs edit is **sequenced behind my structural mod.rs change** (RESOLUTION-B). It must NOT define sweep status types (ledger C3) — I am the single owner. |
| **P-TRANSPORT → P-RECONCILE** | **HARD (file-sequencing only; I am the dependency)** | P-TRANSPORT's single `p2p/mod.rs` event-arm + the 3 additive `P2PConfig` limit fields land AFTER my `run()` refactor — the new event arm slots into my refactored select; the limit fields onto the struct I leave at `:374`. All its other work (behaviour.rs, node, Cargo.toml) is independent. |
| **P-ARC → P-RECONCILE** | **SOFT (I am the dependency, deferred)** | P-ARC's restart-stagger could consume a cross-mesh `StaggerCoordinator` from me; NOT required for v1 (arc ships node-local stagger). Named FOLLOW-ON (§7). |

**Dispatch wave:** WAVE 1 (root). My structural `run()`/`P2PCommand`/snapshot change MUST complete + merge before any Wave-2/3 `p2p/mod.rs` touch (P-TRANSPORT event-arm, P-DIAGNOSTIC bools). Disjoint-file work from other roots runs in parallel; `elohim-compute/src/lib.rs` is the only overlap (additive, RESOLUTION-E).

---

## 5. Task-by-task TDD

### Per-crate build/test discipline (load-bearing — memory)
- **elohim-compute** (native): `RUSTFLAGS=""` `CARGO_TARGET_DIR=/tmp/ec-reconcile` `RUSTC_WRAPPER=""`
- **elohim-storage** (WASM-flagged): `RUSTFLAGS='--cfg getrandom_backend="custom"'` `CARGO_TARGET_DIR=/tmp/es-reconcile` `RUSTC_WRAPPER=""`
- **ts-rs export** (from elohim-views, native): `RUSTFLAGS=""` `CARGO_TARGET_DIR=/tmp/ev-reconcile` `RUSTC_WRAPPER=""`
- **plain `cargo test` — NEVER nextest** (this container has no nextest); never `&&`-pipe a gate exit code — use `2>&1 | tail -N`; `/tmp` target dirs (pool fingerprint-ENOENT); `RUSTC_WRAPPER=""` (sccache spawn-ENOENT).

---

### TASK 1 — `Sweep` trait + `SweepStatus` + `SweepRegistrySnapshot` (S5) in elohim-compute

Files: CREATE `elohim/elohim-compute/src/sweep.rs`; MUTATE `elohim/elohim-compute/src/lib.rs` (append-only re-export block, RESOLUTION-E).

- [ ] Write the failing test FIRST — append to `sweep.rs` a `#[cfg(test)] mod tests` asserting (a) `SweepStatus` serializes camelCase, (b) `SweepRegistrySnapshot { sweeps: vec![..] }` round-trips:
```rust
#[test]
fn sweep_status_serializes_camel_case() {
    let s = SweepStatus {
        name: "drain".into(), enabled: true, cadence_secs: 15,
        last_run_ms: Some(1700), next_tick_ms: Some(1715),
        in_flight: false, runs_total: 3, skips_total: 1, last_error: None,
    };
    let j = serde_json::to_string(&s).unwrap();
    assert!(j.contains("\"cadenceSecs\":15"), "{j}");
    assert!(j.contains("\"lastRunMs\":1700"), "{j}");
    assert!(j.contains("\"runsTotal\":3"), "{j}");
}
#[test]
fn snapshot_wraps_sweeps() {
    let snap = SweepRegistrySnapshot { sweeps: vec![] };
    assert!(serde_json::to_string(&snap).unwrap().contains("\"sweeps\":[]"));
}
```
- [ ] Add to `lib.rs` (append-only, RESOLUTION-E): `pub mod sweep;` and `pub use sweep::{Sweep, SweepStatus, SweepRegistrySnapshot};`. Confirm `ts_rs` + `serde` are already deps of elohim-compute (the existing `report.rs`/`peers.rs` derive `TS`; verify in `Cargo.toml`, add if absent — flag in §7 if a dep add is needed).
- [ ] Run, expect FAIL (module missing), then write `sweep.rs` per §3 S5.
- [ ] Run, expect PASS:
```
cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-reconcile RUSTC_WRAPPER="" cargo test --lib sweep 2>&1 | tail -40
```
- [ ] Regenerate TS (the `#[ts(export_to=...)]` writes `SweepStatus.ts`/`SweepRegistrySnapshot.ts` to `sdk/storage-client-ts/src/generated/`):
```
cd /projects/elohim/elohim/elohim-views && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ev-reconcile RUSTC_WRAPPER="" cargo test export_bindings 2>&1 | tail -20
```
  (If elohim-views does not transitively export elohim-compute's TS types, the export may instead need triggering from elohim-storage's `cargo test export_bindings` once Task 4 embeds the snapshot — verify which crate's harness emits these two files; commit whichever generated files appear. Flag in §7 if neither emits them.)
- [ ] Commit (selective-stage): `git add elohim/elohim-compute/src/sweep.rs elohim/elohim-compute/src/lib.rs sdk/storage-client-ts/src/generated/SweepStatus.ts sdk/storage-client-ts/src/generated/SweepRegistrySnapshot.ts` — message `feat(elohim-compute): Sweep trait + SweepStatus/SweepRegistrySnapshot read-model (S5)`.

---

### TASK 2 — `SweepRegistry` (S6) in elohim-storage

Files: CREATE `elohim/elohim-storage/src/p2p/sweep_registry.rs`; MUTATE `elohim/elohim-storage/src/p2p/mod.rs` (add `pub mod sweep_registry;` near the other `pub mod` declarations at `:38` region).

- [ ] Write the failing test FIRST — append a `#[cfg(test)] mod tests` to `sweep_registry.rs`:
```rust
#[test]
fn register_then_snapshot_reports_defaults() {
    let r = SweepRegistry::new();
    r.register("drain", 15, true);
    let snap = r.snapshot(1000);
    let d = snap.sweeps.iter().find(|s| s.name == "drain").unwrap();
    assert!(d.enabled); assert_eq!(d.cadence_secs, 15);
    assert_eq!(d.runs_total, 0); assert!(d.last_run_ms.is_none());
}
#[test]
fn record_run_advances_counters() {
    let r = SweepRegistry::new();
    r.register("drain", 15, true);
    assert!(r.should_fire("drain"));      // run_now false, but test seam: see note
    r.record_run("drain", 2000);
    let s = r.snapshot(3000).sweeps.into_iter().find(|s| s.name=="drain").unwrap();
    assert_eq!(s.runs_total, 1);
    assert_eq!(s.last_run_ms, Some(2000));
}
#[test]
fn reconfigure_unknown_sweep_refuses_not_actuatable() {
    let r = SweepRegistry::new();
    let err = r.reconfigure("ghost", 30).unwrap_err();
    assert!(matches!(err.code, RefusalCode::NotActuatable));
}
#[test]
fn reconfigure_below_floor_refuses_out_of_bounds() {
    let r = SweepRegistry::new();
    r.register("drain", 15, true);
    assert!(matches!(r.reconfigure("drain", 0).unwrap_err().code, RefusalCode::OutOfGrantBounds));
}
#[test]
fn reconfigure_sets_dirty_and_take_clears() {
    let r = SweepRegistry::new();
    r.register("drain", 15, true);
    r.reconfigure("drain", 30).unwrap();
    assert_eq!(r.take_cadence_change("drain"), Some(30));
    assert_eq!(r.take_cadence_change("drain"), None); // cleared
    assert_eq!(r.snapshot(0).sweeps[0].cadence_secs, 30);
}
#[test]
fn set_enabled_false_records_skip_on_should_fire() {
    let r = SweepRegistry::new();
    r.register("sync", 60, true);
    r.set_enabled("sync", false).unwrap();
    assert!(!r.should_fire("sync"));
    let s = r.snapshot(0).sweeps.into_iter().find(|s| s.name=="sync").unwrap();
    assert!(!s.enabled);
}
#[test]
fn run_now_latch_fires_once() {
    let r = SweepRegistry::new();
    r.register("custody", 120, true);
    r.run_now("custody").unwrap();
    assert!(r.should_fire("custody")); // consumes latch
}
```
  > Note: `should_fire` here exercises only the enabled + run_now latch logic (the timer-due signal comes from the run-loop's `Interval.tick()`, not the registry — the registry's `should_fire` answers "is this sweep allowed to fire right now"). Document this contract in the method doc.
- [ ] Run, expect FAIL, then implement `sweep_registry.rs` per §3 S6. **CONSUMED S1 skip-if-present:** `use elohim_compute::actuation::{ActuationRefusal, RefusalCode};` IF present (verify with `grep -rn "enum RefusalCode" elohim/elohim-compute/src/`); else land the local shim + flag (§7).
- [ ] Run, expect PASS:
```
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-reconcile RUSTC_WRAPPER="" cargo test --lib p2p::sweep_registry 2>&1 | tail -40
```
- [ ] Commit: `git add elohim/elohim-storage/src/p2p/sweep_registry.rs elohim/elohim-storage/src/p2p/mod.rs` — message `feat(elohim-storage): SweepRegistry — cadence + observability + lifecycle (S6)`.

---

### TASK 3 — 3 `P2PCommand` lifecycle variants + `handle_command` arms

Files: MUTATE `elohim/elohim-storage/src/p2p/mod.rs` (enum at `:799`; real `handle_command` arms near `:2807`; in-test drain-stub arms at `:1099`).

- [ ] Write the failing test FIRST — append to the `p2p` test module a test that drives a command through a real `P2PHandle`-style channel OR (simpler, deterministic) asserts the registry mutation directly via a constructed node's `sweep_registry`. Prefer the channel-level test if a test handle exists; else assert `handle_command` routes by calling a small extracted `apply_sweep_command(&self.sweep_registry, cmd)` helper:
```rust
#[tokio::test]
async fn reconfigure_command_replies_ok_and_mutates_registry() {
    let node = /* existing test-node builder, e.g. P2PNode::for_testing(...) */;
    node.sweep_registry.register("drain", 15, true);
    let (tx, rx) = tokio::sync::oneshot::channel();
    node.apply_sweep_command(P2PCommand::ReconfigureCadence {
        sweep: "drain".into(), secs: 30, reply: tx });
    assert!(rx.await.unwrap().is_ok());
    assert_eq!(node.sweep_registry.take_cadence_change("drain"), Some(30));
}
#[tokio::test]
async fn run_now_unknown_replies_refusal() {
    let node = /* test-node */;
    let (tx, rx) = tokio::sync::oneshot::channel();
    node.apply_sweep_command(P2PCommand::RunSweepNow { sweep: "ghost".into(), reply: tx });
    assert!(rx.await.unwrap().is_err());
}
```
  Extract the routing into `fn apply_sweep_command(&self, cmd: P2PCommand)` so it is unit-testable without the full swarm; `handle_command`'s three arms delegate to it (the other ~30 arms keep needing `&mut swarm`, but these three do not — they only touch `self.sweep_registry`).
- [ ] Run, expect FAIL, then add the 3 variants to `P2PCommand` (§3), the 3 real arms in `handle_command` (delegating to `apply_sweep_command`), and the 3 no-op stub arms at the `:1099` in-test match (`reply.send(Ok(()))`). Update any exhaustive `match cmd` elsewhere (grep `match cmd` / `match command` in mod.rs to be safe).
- [ ] Run, expect PASS:
```
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-reconcile RUSTC_WRAPPER="" cargo test --lib p2p:: 2>&1 | tail -40
```
- [ ] Commit: `git add elohim/elohim-storage/src/p2p/mod.rs` — message `feat(elohim-storage): P2PCommand ReconfigureCadence/SetSweepEnabled/RunSweepNow (closes T22-live-reconfig)`.

---

### TASK 4 — Embed `SweepRegistrySnapshot` in `P2PStatusInfo` + populate

Files: MUTATE `elohim/elohim-storage/src/p2p/mod.rs` (`P2PStatusInfo` at `:707`; `refresh_status()` literal at `:7050`; the two stub literals at `:1152`, `:1692`).

- [ ] Write the failing test FIRST — append:
```rust
#[test]
fn p2p_status_info_carries_sweeps_snapshot() {
    // build a full P2PStatusInfo literal (copy the :1152 stub) with a
    // non-empty snapshot, assert camelCase wire key.
    let info = /* P2PStatusInfo { .., sweeps: SweepRegistrySnapshot { sweeps: vec![ /* one */ ] } } */;
    let j = serde_json::to_string(&info).unwrap();
    assert!(j.contains("\"sweeps\""), "{j}");
}
```
- [ ] Run, expect FAIL, then add `pub sweeps: SweepRegistrySnapshot,` to `P2PStatusInfo` (after `placement_gaps_emitted_total` at `:756` — leaving the diagnostic's 2 anchor bools to land AFTER, per §7 seam ordering). Populate in `refresh_status()`: `sweeps: self.sweep_registry.snapshot(now_ms())` (add a small `now_ms()` helper = `SystemTime::now()` epoch-ms). Add `sweeps: SweepRegistrySnapshot { sweeps: vec![] }` to the `:1152` and `:1692` stub literals.
- [ ] Run, expect PASS (same `p2p::` command).
- [ ] Regenerate TS (this is the storage-owned struct, so trigger from the storage/views export harness): `cd /projects/elohim/elohim/elohim-views && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ev-reconcile RUSTC_WRAPPER="" cargo test export_bindings 2>&1 | tail -20` — confirm `P2PStatusInfo.ts` (and the `p2p-status-view.ts` consumer if regenerated) gains `sweeps`. **Verify byte-stable diff** — only `sweeps` added (memory: codegen oscillation is cosmetic; do not churn other types).
- [ ] Update `elohim/sdk/schemas/v1/views/p2p-status-view.schema.json` additively — add a `sweeps` object (`{ "type":"object", "properties": { "sweeps": {"type":"array", "items": {...SweepStatus...}} }, "required":["sweeps"] }`) to `properties` + `"sweeps"` to top-level `required`, so the existing `tests/schema_contract.rs::p2p_status_view_matches_schema` passes.
  > **HAND-OFF to P-DIAGNOSTIC + P-PROOFS:** P-DIAGNOSTIC also edits this schema (adds `selfCidPresent`/`provideLoopEnabled`) and `schema_contract.rs`; P-PROOFS owns NEW `tests/` files. My edit is additive + sequenced first (Wave 1); their additive edits rebase onto mine. Mechanical merge.
- [ ] Run schema contract: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-reconcile RUSTC_WRAPPER="" cargo test --test schema_contract p2p_status 2>&1 | tail -40`.
- [ ] Commit: `git add elohim/elohim-storage/src/p2p/mod.rs elohim/sdk/schemas/v1/views/p2p-status-view.schema.json sdk/storage-client-ts/src/generated/P2PStatusInfo.ts app/elohim-app/src/app/generated/p2p-status-view.ts` — message `feat(elohim-storage): embed SweepRegistrySnapshot in P2PStatusInfo (S9)`.

---

### TASK 5 — Structural `run()` refactor: route periodic arms through the registry (S9)

> **This is the deep change.** It MUST preserve every existing arm's behavior bit-for-bit (cadences, `sync_paused` gates, missed-tick=Skip, the `pending()` disable trick for inventory/custody, the drain-arm's status-refresh + auto-suppress). The ONLY net new behavior: register each periodic sweep, count runs/skips, and check `should_fire` + `take_cadence_change` so live reconfigure works.

Files: MUTATE `elohim/elohim-storage/src/p2p/mod.rs` (`run()` at `:2197`; add `sweep_registry` to `P2PNode` struct `:470` region + both constructors `:1093`-path and `:1690`-path).

- [ ] Write the failing/observable test FIRST — a `#[tokio::test]` (or extend an existing run-loop test) using `tokio::time::pause`/`advance` to assert that after one interval the registry reports `runs_total >= 1` for a fast-cadence sweep, AND that a `ReconfigureCadence` sent before the second tick changes the observed cadence in the snapshot:
```rust
#[tokio::test(start_paused = true)]
async fn run_loop_records_sweep_runs_in_registry() {
    let node = /* test node with a short status cadence */;
    let (sd_tx, sd_rx) = tokio::sync::broadcast::channel(1);
    let handle = tokio::spawn({ let n = node.clone(); async move { n.run(sd_rx).await } });
    tokio::time::advance(Duration::from_secs(31)).await; // one status tick
    let snap = node.sweep_registry.snapshot(now_ms());
    assert!(snap.sweeps.iter().any(|s| s.name=="status" && s.runs_total >= 1));
    let _ = sd_tx.send(());
    let _ = handle.await;
}
```
  (If a clonable test node is not available, assert at the seam: factor the arm bodies' registry calls so the registry mutation is observable via a smaller harness. Keep the test deterministic with `start_paused`.)
- [ ] Run, expect FAIL, then implement:
  1. Add `sweep_registry: Arc<sweep_registry::SweepRegistry>` to `P2PNode` struct + both constructors (initialize empty `Arc::new(SweepRegistry::new())`).
  2. At `run()` entry, AFTER the existing `let mut X_interval = interval(..)` declarations, **register each periodic sweep** with its resolved cadence + enabled flag:
     `self.sweep_registry.register("status", 30, true);` … through all eleven (`sync`, `replication`, `gap_dispatch`, `acquisition_reconcile`, `provide_reconcile`, `acquisition_dispatch`, `drain`, `bootstrap_retry`, `inventory_broadcast` (enabled = `inventory_broadcast_interval.is_some()`), `custody_sweep` (enabled = `custody_sweep_interval.is_some()`)). Keep the existing `Interval` locals — they remain the timer source; the registry is the schedule/observability layer alongside them.
  3. In EACH periodic `_ = X_interval.tick() => { .. }` arm: (a) `drop(swarm)` as today; (b) at the top, `if !self.sweep_registry.should_fire("X") { self.sweep_registry.record_skip("X"); continue/return-from-arm }` — **but** preserve the existing `sync_paused` skip as a `record_skip` too (don't double-fire the body); (c) run the existing body verbatim; (d) `self.sweep_registry.record_run("X", now_ms())` after the body. Where an arm already has `if self.sync_paused { skip } else { body }`, the skip branch calls `record_skip` and the body branch ends with `record_run`.
  4. **Live reconfigure:** at the top of the loop (before `select!`), poll `for name in [..]: if let Some(secs) = self.sweep_registry.take_cadence_change(name) { /* re-arm the matching Interval: *X_interval = interval(Duration::from_secs(secs)); set Skip */ }`. This closes `TODO(T22-live-reconfig)` — DELETE that TODO comment block (`:2249–2255`).
  5. `run_now` is honored automatically: `should_fire` returns true when the latch is set even if the timer hasn't ticked — BUT the timer arm only runs on `.tick()`. To make `RunSweepNow` actually fire off-cadence, add a single `run_now_poll_interval = interval(1s)` arm that checks each sweep's `run_now` latch via `should_fire` and dispatches the matching body once. (Keep this minimal: a `match name { "drain" => self.drain_publish_queue(500).await, .. }` dispatch table reused by both the timer arms and the run-now poll — extract a `dispatch_sweep_body(&mut swarm, name)` helper to avoid duplicating the 11 bodies.)
  > Extracting `dispatch_sweep_body` is the clean refactor: each timer arm becomes `{ drop(swarm); if should_fire { dispatch_sweep_body(name).await; record_run } else { record_skip } }`, and the run-now poll reuses it. This is the structural simplification finding #4 asks for — eleven bespoke arms collapse to a uniform shape over a dispatch table.
- [ ] Run, expect PASS:
```
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-reconcile RUSTC_WRAPPER="" cargo test --lib p2p:: 2>&1 | tail -60
```
- [ ] Commit: `git add elohim/elohim-storage/src/p2p/mod.rs` — message `refactor(elohim-storage): route run() periodic sweeps through SweepRegistry (S9; closes #4 cadence scatter)`.

---

### TASK 6 — Config cadence fields (RESOLUTION-C, storage Config)

Files: MUTATE `elohim/elohim-storage/src/config.rs` (additive fields + default fns + `Default` impl lines).

- [ ] Write the failing test FIRST — assert the new fields default + deserialize:
```rust
#[test]
fn config_sweep_cadence_defaults() {
    let c = Config::default();
    assert_eq!(c.status_sweep_seconds, 30);
    assert_eq!(c.drain_sweep_seconds, 15);
    // ... the cadence knobs that don't already exist
}
```
- [ ] Run, expect FAIL, then add ONLY the cadence fields that are not already present (note: `inventory_broadcast_seconds` and `custody_sweep_seconds` ALREADY exist at `:141`/`:145` — do NOT re-add; the registry reads those existing fields for those two sweeps). Add disjoint-named fields for the currently-hardcoded cadences I want operator-tunable (`status_sweep_seconds=30`, `sync_sweep_seconds=60`, `replication_sweep_seconds=60`, `drain_sweep_seconds=15`, etc.), each `#[serde(default = "default_X")]` + matching `Default` impl line + a `fn default_X() -> u64`. **Additive-only — no method rewrites, no reordering** (RESOLUTION-C; integrator merges any literal conflict mechanically).
  > Keep this MINIMAL: only promote cadences to config where live-tunability via `ReconfigureCadence` plus a config seed adds value. Cadences that should stay code-constant (e.g. bootstrap-retry's backoff) keep their literals and register with the literal. State which are config-seeded vs literal-seeded in a comment.
- [ ] Run, expect PASS:
```
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-reconcile RUSTC_WRAPPER="" cargo test --lib config 2>&1 | tail -30
```
- [ ] Wire the config seeds into the `register(..)` calls in `run()` (Task 5) — replace the literal `30`/`60`/`15` cadences with `self.config.status_sweep_seconds` etc. Re-run the Task 5 run-loop test.
- [ ] Commit: `git add elohim/elohim-storage/src/config.rs elohim/elohim-storage/src/p2p/mod.rs` — message `feat(elohim-storage): operator-tunable sweep cadences in Config (RESOLUTION-C additive)`.

---

### TASK 7 — Whole-crate gates (both crates)

- [ ] Run, expect green:
```
cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-reconcile RUSTC_WRAPPER="" cargo test --lib 2>&1 | tail -30
cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-reconcile RUSTC_WRAPPER="" cargo clippy -- -D warnings 2>&1 | tail -30
cd /projects/elohim/elohim/elohim-compute && cargo fmt --check
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-reconcile RUSTC_WRAPPER="" cargo test --lib 2>&1 | tail -40
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-reconcile RUSTC_WRAPPER="" cargo clippy --lib --tests -- -D warnings 2>&1 | tail -40
cd /projects/elohim/elohim/elohim-storage && cargo fmt --check
```
- [ ] No commit (verification only). If clippy flags the new code, fix in-place and amend the relevant task commit.

---

## 6. p2p-class (cite the class; ledger §p2p-design-gate)

- **Registry snapshot read-model = Cat-C node-local read-model.** `SweepStatus`/`SweepRegistrySnapshot` are projections of in-process operational scheduling state — NO DHT entry type, NO table, NO content-addressed identity, NO coordinator function, NO new HTTP route (they ride the existing `/p2p/status` → `P2PStatusInfo` surface). Per the ledger: "New runtime entities are Cat-C node-local read-models … Do not re-litigate; cite the class." The swap-test holds: any node scheduling the same sweeps reports the same snapshot shape.
- **The lifecycle COMMANDS that MUTATE cadence are Cat-A (actuation spine), but I only expose the `P2PCommand` surface.** Reconfigure/pause/run-now are *actuations* of a node knob — the canonical authority path for them is **P-ACTUATION's `Actuation` contract + REA `Commitment` grant-bounds** (Cat-A, notarized, `sets-authority-arc`-style). I do NOT build that authority path; I expose the **mechanism** (the 3 `P2PCommand` variants returning `ActuationRefusal`) so the actuation spine can drive it. The §7 actuation seam names where an `impl Actuation for SweepCadenceKnob` would terminate (its `render()` sends `P2PCommand::ReconfigureCadence`). For v1 the commands are reachable via the internal handle only (no public unauthenticated route) — the floor refusal (`secs < 1`) is the only rail until the Cat-A grant path lands. This is the correct split: **Cat-C read, Cat-A actuate, mechanism-only here.**

---

## 7. `// FOLLOW-ON seams` (left for the integration pass / named owners)

```rust
// FOLLOW-ON(P-ACTUATION-integration): if the S1 ActuationRefusal local shim
//   was landed in sweep_registry.rs (P-ACTUATION's lib.rs re-export slipped),
//   DELETE it and import elohim_compute::actuation::{ActuationRefusal,RefusalCode}.
//   Integration §2.2 greps for exactly one `enum RefusalCode` — mine must be gone.

// FOLLOW-ON(P-ACTUATION): SweepCadenceKnob as an Actuation instance. An
//   `impl Actuation for SweepCadenceKnob { type Effect = P2PCommand; }` whose
//   render() emits P2PCommand::ReconfigureCadence, gated by a ScopeId::SweepCadence(name)
//   REA grant. NOT built here — I expose the mechanism; the Cat-A authority path
//   is P-ACTUATION's. The registry's hard floor (secs>=1) is the only rail in v1.

// FOLLOW-ON(P-DIAGNOSTIC): P2PStatusInfo gains self_cid_present/provide_loop_enabled
//   AFTER my `sweeps` field (RESOLUTION-B sequencing). They are orthogonal struct
//   additions; the integrator merges both into each P2PStatusInfo literal.

// FOLLOW-ON(P-TRANSPORT): the connection_limits event-arm slots into my
//   refactored select! AFTER this lands; the 3 P2PConfig limit fields onto the
//   struct at mod.rs:374. My run() refactor leaves the select! shape uniform so
//   the new arm appends cleanly.

// FOLLOW-ON(P-ARC, SOFT, deferred v2): cross-mesh StaggerCoordinator — P-ARC's
//   restart-stagger could register as a sweep / consume a coordinator from the
//   registry. Node-local stagger suffices for v1 (integration §2.1 DEFER).

// FOLLOW-ON(integration): RunSweepNow off-cadence dispatch uses a 1s poll arm.
//   If a future need for sub-second run-now precision arises, replace the poll
//   with a tokio::Notify per sweep. v1's 1s latency is acceptable for an
//   operator-driven incident-response trigger.

// FOLLOW-ON(integration): if elohim-compute's Cargo.toml lacked ts_rs/serde as a
//   direct dep, the dep-add is flagged here (Task 1). Confirm the TS export
//   harness (elohim-views vs elohim-storage) emits SweepStatus.ts /
//   SweepRegistrySnapshot.ts — commit whichever crate's `cargo test export_bindings`
//   produces them.
```

---

## 8. Dispatch note

- **Isolated worktree, subagent-driven, commit-only.** Run from a dedicated worktree off the integration branch. The integrator pushes/merges — **never `git push`** (memory: commit-only; integrator owns push/merge).
- **I am a WAVE-1 ROOT and the PRIMARY structural owner of `p2p/mod.rs`.** My structural `run()`/`P2PCommand`/snapshot change MUST land + merge before any Wave-2/3 `p2p/mod.rs` touch (P-TRANSPORT event-arm, P-DIAGNOSTIC anchor bools). Finish Tasks 3–5 (the mod.rs surgery) early so siblings can rebase.
- **Task ordering inside the worktree:** Task 1 (compute S5) → Task 2 (registry S6) → Task 3 (commands) → Task 4 (snapshot embed) → Task 5 (run() refactor — the deep change, do it on a clean tree) → Task 6 (config) → Task 7 (gates). Tasks 1–2 have no mod.rs dependency and could pre-start; Tasks 3–6 are mod.rs-heavy and serial within this worktree.
- **Selective-stage each commit** (concurrent sessions may share the worktree — memory): the per-task `git add` lists name exact files only; never bulk-revert ambient mods.
- **Per-crate RUSTFLAGS discipline is load-bearing:** `--cfg getrandom_backend="custom"` for elohim-storage (WASM), `""` for elohim-compute / elohim-views (native — flag leak → `undefined __getrandom_v03_custom` at link). `RUSTC_WRAPPER=""`, `/tmp` target dirs, plain `cargo test`, never `&&`-pipe a gate exit code.
- **NEVER-TOUCH guards (§2) are non-goals** — GapTracker requeue, DispatchBudget credits, `in_scope_of` upsert, DedupLru/WriteThroughState poison, `sync_paused` skip semantics. I schedule + observe + expose lifecycle; I do not alter any sweep's body logic or any flow-control invariant.
