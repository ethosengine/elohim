# Dataplane Actuation — Canonical Contract & Arc-As-Instance (P-ACTUATION)

> Working draft. NOT cite-sealed. For agentic workers: REQUIRED SUB-SKILL: superpowers:subagent-driven-development (or executing-plans). Steps use checkbox (- [ ]) syntax.
>
> Track id in the 2026-06-14 P2P-Dataplane Contract Ledger: **P-ACTUATION** (WAVE 1 root — ZERO inbound hard edges). Authored against `/projects/elohim/P2P-DATAPLANE-CONTRACT-LEDGER-2026-06-14.md` (THE LAW) and `/projects/elohim/P2P-DATAPLANE-INTEGRATION-2026-06-14.md` (G0-A/G0-C/G0-D must-fixes). Obey their file-ownership, single-owners, interface names, and dependency edges exactly.

---

## 1. CONTEXT / WHY + FINDING IT CLOSES

**This is one of the TWO WAVE-1 ROOTS the whole 7-plan set depends on.** P-ARC HARD-consumes S1/S2/S13 from here; P-RECONCILE's three lifecycle `P2PCommand`s return this track's `ActuationRefusal` (S1); the mishpat zome consumes this track's `ScopeId` strings (S3). P-ACTUATION has **zero inbound hard edges** — it promotes arc's *existing* enums into `elohim-compute` as new code, never importing from arc.

**Finding closed — #5 actuator-pattern drift.** The self-healing control plane shipped a *bespoke* actuation core (`arc_actuator.rs`: `authorize` + `coverage_admits` + `render_conductor_arc_factor` + `parse_grant_bounds` + the apply-shell) AND a private `ActuationRefusal`/`RefusalCode` vocabulary used only by `http.rs`. The reconcile track wants the same refuse+elevate vocabulary for its sweep lifecycle commands; the transport track will want runtime-tunable connection limits; the defense track rejected a bespoke `rate_limit_rpm` limiter on the grounds that *any* future knob must be an instance of one contract. Today there is **no contract** — every actuatable knob would re-invent the refuse/authorize/bounds/render shape, and the scope string `"conductor.target_arc_factor"` is **literally duplicated** in the mishpat zome (`commitments.rs:511`) and the actuator (`arc_actuator.rs` tests). This plan **canonicalizes ONE `Actuation` contract** in `elohim-compute`, makes **arc the first instance** (proving the contract against already-built code), and **kills the duplicated literal** via a `scope_vocab`. It also lands the single highest-leverage fix in the whole campaign: the **`sets-authority-arc` projection arm** (S13) — without it, the shipped arc actuate path is DEAD (the projection falls to `other =>` → empty bounds → state never `active` → `http.rs:2641` returns 409 "grant is not active" forever).

**Model the contract on `WriteThroughState` (S10), do NOT import it.** `write_through.rs:225-336` is the runtime-mutable admin-override TEMPLATE: a 4-layer resolution stack (manifest → policy → env → live admin), the live layer behind an `Arc<RwLock<...>>` with **poison-tolerant** reads/writes (`.unwrap_or_else(|poison| poison.into_inner())`, `:314`/`:322`/`:344`), and an **integrity-kind short-circuit** that *cannot be overridden off* (`:336` `is_integrity_kind` returns before any override layer). The `Actuation` contract mirrors this SHAPE (typed RwLock-live override, poison-tolerant, a never-touch short-circuit) — it does **not** import or generalize `WriteThroughState` (S10 stays write-through-specific, ledger §1).

**Already-built — verify-only, never re-author** (ledger §5): `arc_policy.rs` (`derive`), `arc_actuator.rs` (the decision core + the TODAY-home of `ActuationRefusal`/`RefusalCode`), `system_metrics::container_memory_limit_bytes`, `http.rs` `GET/POST /api/v1/status/arc-policy`, the `sets-authority-arc` DNA validator (`commitments.rs:481`, DNA-hash-NEUTRAL, cad5fb67c). This plan **moves and refactors**; it does not rebuild option (i).

---

## 2. OWNED FILES (verbatim from the Contract Ledger §2 P-ACTUATION block)

This plan creates/mutates EXACTLY:

- **C** `elohim/elohim-compute/src/actuation.rs` (`Actuation`, `GrantBounds`, `ActuationRefusal`, `RefusalCode`, `MetaCooldown`, `NeverTouch`) — SOLE owner.
- **C** `elohim/elohim-compute/src/scope_vocab.rs` (`ScopeId` + `scope_str()`) — SOLE owner.
- **C** `elohim/elohim-storage/src/services/actuation/mod.rs` + `elohim/elohim-storage/src/services/actuation/arc.rs` (arc-instance home; receives P-ARC's `StaggerGate` hand-off) — SOLE owner of the new dir.
- **M** `elohim/elohim-storage/src/services/arc_actuator.rs` → thin re-export shim (preserve the `http.rs` call sites) — see RESOLUTION-A. SOLE mutator.
- **M** `elohim/elohim-storage/src/mishpat_projection.rs` (new `sets-authority-arc` match arm `parse_sets_authority_arc`) — SOLE owner of this match (S13).
- **M** `elohim/elohim-storage/src/http.rs` (point `handle_arc_policy_actuate` at the new contract) — RESOLUTION-F. SOLE owner of the actuate-path repoint.
- **M** `elohim/elohim-compute/src/lib.rs` (+re-exports: `actuation`, `scope_vocab`) — RESOLUTION-E, append-only.
- **M** `elohim/elohim-storage/src/services/mod.rs` (add `pub mod actuation;` next to the existing `arc_actuator`/`arc_policy` lines at :25-26) — additive, P-ACTUATION-local.
- **M** `elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs` (consume `scope_vocab` strings — replace the literal `"conductor.target_arc_factor"` at `:511`) — SOLE owner of this edit.

> **Ledger drift correction (SEAM-DELTA):** none for P-ACTUATION's own files (G0-D's two stale paths belong to P-ARC's `services/system_metrics.rs` and P-PROOFS' edge Jenkinsfile — not ours). Confirmed: `elohim-compute` is ALREADY a path dep of `elohim-storage` (`Cargo.toml:48`); `arc_actuator`'s only external consumers are `http.rs:2594` + `http.rs:2680` (grep-verified) — the shim need only preserve those two call sites.

**Explicit non-touch / NEVER-TOUCH guards (read-only to any actuation — ledger §1 + the prompt's never-touch list):**
- **`WriteThroughState` integrity-kind short-circuit can't be overridden off** (`write_through.rs:336`). The contract MODELS this shape (a `NeverTouch` short-circuit) but does NOT modify `write_through.rs`. P-ACTUATION does not touch `write_through.rs` at all.
- **`DedupLru`/`WriteThroughState` divergent poison policies must NOT be harmonized.** They intentionally differ; this plan copies `WriteThroughState`'s poison-tolerant pattern into the *new* `arc.rs` override cell only — it does not reach into either existing type.
- **`in_scope_of` upsert** and **`GapTracker` never-immediate-requeue** — out of this plan's file list entirely; listed here as explicit non-goals so no task drifts into them.
- **`config.rs` is NOT touched** (RESOLUTION-C is for transport/reconcile/node; arc reads `system_metrics`, not config).
- **`process_manager.rs` is NOT touched by P-ACTUATION** — P-ARC lands `StaggerGate`/`may_arc_restart` there (Task 3 of P-ARC); P-ACTUATION only *calls* `may_arc_restart` from `services/actuation/arc.rs` (the receiver, G0-C).

**Collision statement:** Every shared file is an append-only seam resolved by the ledger: `elohim-compute/src/lib.rs` (RESOLUTION-E, one block); `http.rs` (RESOLUTION-F, P-ACTUATION is SOLE actuate-path owner — P-ARC's ONE additive `local_authored_bytes` input line at `http.rs:2764` is disjoint, in `handle_arc_policy_status`, not the actuate handler; P-DIAGNOSTIC does NOT touch storage `http.rs`). `arc_actuator.rs` is P-ACTUATION-only (RESOLUTION-A; P-ARC must NOT touch it). The `mishpat_projection.rs` `sets-authority-arc` arm is touched by no other plan.

---

## 3. PRIMITIVES — OWNED vs CONSUMED

### OWNED by P-ACTUATION (single owner; others consume)

All signatures below are the **canonical** ledger §3 rows — match verbatim. Home crate is `elohim-compute` (the shared crate; zero new crates).

**S2 — `trait Actuation` + `GrantBounds`** (`elohim_compute::actuation`)

```rust
/// The canonical actuation contract. Every actuatable knob in the dataplane
/// (arc, sweep cadence, connection limits, admission) is an INSTANCE of this —
/// one refuse/authorize/render shape, not a per-knob re-invention (finding #5).
/// Modeled on the WriteThroughState 4-layer override SHAPE (S10) but importing
/// nothing from it.
pub trait Actuation {
    /// The concrete effect this actuator renders (e.g. a rewritten config YAML).
    type Effect;

    /// Stage 1 — is this request authorized by the grant? (REA bounds + expiry).
    /// Pure: clock injected as `now` (unix seconds).
    fn authorize(
        &self,
        req: &ActuationReq,
        bounds: &GrantBounds,
        now: u64,
    ) -> Result<(), ActuationRefusal>;

    /// Stage 2 — does the live operational gate admit it? (coverage floor,
    /// meta-cooldown, never-touch). Pure over the injected `GateCtx` snapshot.
    fn gate(&self, req: &ActuationReq, ctx: &GateCtx) -> Result<(), ActuationRefusal>;

    /// Stage 3 — render the effect (still pure; the impure apply is the caller's).
    fn render(&self, req: &ActuationReq, ctx: &GateCtx) -> Result<Self::Effect, ActuationError>;
}

/// A generic actuation request: the scope being actuated + a typed value + the
/// authorizing commitment CID (CID = entry_hash, gospel
/// `project_mishpat_commitment_cid_is_entry_hash`).
#[derive(Debug, Clone)]
pub struct ActuationReq {
    pub scope: ScopeId,
    /// The proposed value, scope-interpreted (arc: {0,1}; cadence: seconds; etc.).
    pub value: u64,
    pub commitment_cid: String,
}

/// The REA delegates-compute grant scope/expiry envelope. Generalizes the
/// arc-specific `ArcGrantBounds` (min/max + a domain floor + expiry + scope).
#[derive(Debug, Clone, Copy)]
pub struct GrantBounds {
    pub min: u64,
    pub max: u64,
    /// Scope-interpreted operational floor (arc: the per-key coverage `r_floor`).
    pub domain_floor: u32,
    /// Grant expiry (unix seconds); 0 = no expiry encoded (matches the
    /// `Option<u64>::None` arc semantics, flattened for Copy ergonomics).
    pub expires_at_epoch_s: u64,
    pub scope: ScopeId,
}

/// Live operational snapshot the `gate`/`render` stages read (clock + the
/// scope-specific observed state). Injected → pure/testable.
#[derive(Debug, Clone)]
pub struct GateCtx {
    pub now: u64,
    /// Observed mesh size for coverage gates (arc: conductor peer count).
    pub observed_n: u32,
    /// The meta-cooldown guard for this scope (rate-limit-the-rate-limiter).
    pub cooldown: MetaCooldown,
    /// Never-touch rails active for this scope (refuse if any matches).
    pub never_touch: NeverTouch,
}

/// Errors from the impure render/apply layer (kept separate from refusals,
/// which are POLICY outcomes). Generalizes `arc_actuator::ActuationError`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActuationError {
    /// The render target had no place to write the effect (arc: no `network:`).
    NoRenderTarget,
    /// A scope-specific render failure with a message.
    Render(String),
}
```

**S1 — `ActuationRefusal` + `RefusalCode`** (`elohim_compute::actuation`) — PROMOTED verbatim from `arc_actuator.rs:77-92`, plus the generic `GateRefused(String)` variant the ledger mandates (so P-RECONCILE's command replies don't invent a parallel `ReconfigureError` — audit C2):

```rust
/// A refusal carrying a finding to ELEVATE — the runtime form of "do not shrink
/// into a keyspace gap / not actuatable / gate refused". The exact payload the
/// self-healing elevate/finding sink consumes (machine `code` + human `elevate`).
/// PROMOTED from arc_actuator (the single-owner move; nobody redefines).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActuationRefusal {
    pub code: RefusalCode,
    /// The finding to elevate to a human/operator (kept for compat with the
    /// arc call sites that read `.elevate` as a String).
    pub elevate: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefusalCode {
    /// Proposed value is outside the grant's `[min,max]`.
    OutOfGrantBounds,
    /// The grant has expired.
    GrantExpired,
    /// Proposed value is not an actuatable value (arc: only `{0,1}` exist).
    NotActuatable,
    /// A scope-specific operational gate refused, with a message. Subsumes arc's
    /// `WouldBreakCoverage` AND P-RECONCILE's sweep-lifecycle refusals AND the
    /// meta-cooldown/never-touch rails (one generic refusal, audit C2).
    GateRefused(String),
}
```

> **Compat note (load-bearing for the shim):** arc's TODAY `ActuationRefusal.elevate` is a bare `String`; the canonical promotes it to `Option<String>`. The shim (Task 4) re-exports the canonical type AND provides a `From`/adapter so `http.rs:2701` (`ActuationRefusal { code, elevate }`) still destructures — see Task 4. arc's `WouldBreakCoverage` code becomes `GateRefused("…coverage…")` at the call site; the elevate string carries the human detail unchanged.

**S3 — `ScopeId` + `scope_str()`** (`elohim_compute::scope_vocab`):

```rust
/// The canonical knob/scope-string namespace. Kills the literal
/// `"conductor.target_arc_factor"` duplicated in the mishpat zome
/// (commitments.rs:511) and the actuator. Every actuatable scope names itself
/// here; `scope_str()` is the ONE wire/validator string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScopeId {
    /// The conductor authority-arc factor (the FIRST instance — arc).
    NetworkArc,
    /// doorway warmup timeout knob family (FOLLOW-ON; deferred tool surface).
    DoorwayWarmupTimeout,
    /// storage admission concurrency (FOLLOW-ON; deferred tool surface).
    StorageAdmission,
    /// A reconcile sweep cadence, by sweep name (P-RECONCILE consumer).
    SweepCadence(&'static str),
}

impl ScopeId {
    /// The single canonical wire/validator string for this scope.
    pub fn scope_str(&self) -> &'static str {
        match self {
            ScopeId::NetworkArc => "conductor.target_arc_factor",
            ScopeId::DoorwayWarmupTimeout => "doorway.warmup.timeout",
            ScopeId::StorageAdmission => "storage.admission",
            ScopeId::SweepCadence(name) => name, // "sweep.<id>"
        }
    }
}
```

> The zome (`commitments.rs:511`) compares `bounds["knob"]` against `ScopeId::NetworkArc.scope_str()` — but the zome is a `wasm32-unknown-unknown` crate that must NOT take a heavy native dep. **Decision (Task 6):** add `scope_vocab` to `elohim-compute` as a `#![no_std]`-friendly, dependency-free module (it is just an enum + `match` returning `&'static str` — no std beyond `core`). Verify at build time the zome can depend on `elohim-compute` with `default-features = false`; if `elohim-compute` cannot be made wasm-safe cheaply, the zome instead gets a **single `pub const ARC_KNOB: &str`** re-exported from a tiny shared location and the storage side imports the same const — the deduplication target is "one definition," not necessarily "one crate." Task 6 picks the cheaper wasm-safe path and records it.

**S4 — `MetaCooldown` + `NeverTouch`** (`elohim_compute::actuation`) — DESIGN §7 rails as types:

```rust
/// Rate-limit-the-rate-limiter guard: a minimum interval between actuations per
/// scope, so an actuation storm cannot itself overwhelm (the cure must not cause
/// the partition). Pure: `admits(now)` is a clock comparison.
#[derive(Debug, Clone, Copy)]
pub struct MetaCooldown {
    pub min_interval_s: u64,
    pub last_actuated: Option<u64>,
}

impl MetaCooldown {
    pub fn new(min_interval_s: u64) -> Self {
        Self { min_interval_s, last_actuated: None }
    }
    /// Ok(()) if `now` is at least `min_interval_s` past the last actuation.
    /// Err(remaining_secs) to DEFER (never force).
    pub fn admits(&self, now: u64) -> Result<(), u64> {
        match self.last_actuated {
            None => Ok(()),
            Some(last) => {
                let elapsed = now.saturating_sub(last);
                if elapsed >= self.min_interval_s { Ok(()) }
                else { Err(self.min_interval_s - elapsed) }
            }
        }
    }
    /// Record a successful actuation (caller updates after apply).
    pub fn mark(&mut self, now: u64) { self.last_actuated = Some(now); }
}

/// Marker for never-touch rails: scopes/effects an actuation may NEVER override.
/// The TYPE form of the prompt's never-touch list (integrity-kind short-circuit,
/// divergent poison policies, in_scope_of upsert, GapTracker requeue). A gate
/// that matches a never-touch rail returns RefusalCode::GateRefused. Default =
/// empty (no rail) so non-arc instances opt in explicitly.
#[derive(Debug, Clone, Default)]
pub struct NeverTouch {
    /// Human-named rails this gate must refuse (e.g. "integrity-kind").
    pub rails: Vec<&'static str>,
}

impl NeverTouch {
    /// Returns the first matching rail name, if the request would breach one.
    pub fn breached_by(&self, _req: &ActuationReq) -> Option<&'static str> {
        // arc instance: no rail breach is possible (it only sets the arc factor,
        // not an integrity kind). FOLLOW-ON instances populate `rails` and the
        // match. The arc instance ships with an EMPTY NeverTouch.
        None
    }
}
```

**S13 — the `sets-authority-arc` projection arm** (`mishpat_projection.rs`, new `parse_sets_authority_arc`). Currently `sets-authority-arc` falls to `other =>` (`:180`) → empty bounds → `state: "proposed"` → `http.rs:2641` rejects forever. The new arm projects the grant with **real bounds** AND `state: "active"` so the actuate path is reachable. **Single highest-leverage fix.** (See Task 5 for the `active` semantics.)

**The typed/MCP tool surface stubs** (`services/actuation/mod.rs`) — DEFINE the surface; mark deferred follow-ons:

| Tool | Status | Maps to scope |
|---|---|---|
| `re_derive_auto` / arc actuate (`tune_knob` for `NetworkArc`) | **ACTIVE (this plan)** — the arc instance | `ScopeId::NetworkArc` |
| `tune_knob` (generic) | SURFACE DEFINED; arc-only wired | `ScopeId::*` |
| `pause_sweep` / `resume_sweep` / `run_now` | **FOLLOW-ON** — owned by P-RECONCILE's `P2PCommand`s; this plan only reserves the `ScopeId::SweepCadence` namespace | `ScopeId::SweepCadence(_)` |
| `reset_connection` / `adjust_flow_window` | **FOLLOW-ON (deferred)** — needs a transport instance | `ScopeId::*` (TBD) |
| `quarantine_peer` / `readmit_peer` | **FOLLOW-ON (deferred)** — peer-lifecycle instance | (TBD) |

The tool surface is a documented `enum ActuationTool { … }` + a `dispatch` stub that wires ONLY `NetworkArc` and returns `RefusalCode::NotActuatable` (with an elevate naming the owning follow-on plan) for every deferred tool. No MCP wiring is built tonight — the surface is the contract; the deferred arms are explicit `// FOLLOW-ON` seams.

### CONSUMED (with the skip-if-present clause)

> **Skip-if-present rule (verbatim, ledger §1):** "Before landing this type, verify `elohim-compute` (or the named owner module) already exposes it. If present, VERIFY-ONLY (import + use). If absent at your integration point, land the owner-plan's verbatim definition only as a temporary local shim, flag it in your plan's hand-off notes, and delete the shim when the owner lands."

- **S10 `WriteThroughState` override-layer pattern** — already shipped (`write_through.rs`). CONSUME the SHAPE only (the 4-layer + poison-tolerant `Arc<RwLock>` + never-touch short-circuit). VERIFY-ONLY read; **do NOT import the type** (it stays write-through-specific).
- **S12 `Mishpat::Commitment` / `sets-authority-arc` action** — already shipped (DNA, cad5fb67c). VERIFY-ONLY; this plan fixes its *projection* (S13), never the DNA entry/validator structure (only the literal-string source at `:511`).
- **S8 `CircuitBreaker`/`CircuitState`** — already shipped (`elohim_compute::peers`, re-exported `lib.rs:19`). MODEL reference only (the single-owner precedent this plan mirrors). Not imported.
- **P-ARC's `StaggerGate` / `ConductorManager::may_arc_restart`** — OWNER P-ARC (lands in `process_manager.rs`, P-ARC Task 3). **RECEIVED here** (G0-C): `services/actuation/arc.rs`'s apply path calls `conductor.may_arc_restart(now, min_interval)` BEFORE `restart()`. **Skip-if-present / shim clause:** if P-ARC's `StaggerGate` has not landed at integration time, the arc apply path uses the in-contract `MetaCooldown` as the local stagger guard (functionally equivalent — both are "min interval between actuations, defer not force") and flags the seam; when P-ARC's `may_arc_restart` lands, the integrator swaps the `MetaCooldown` guard for the call (one site). Either way the gate is NOT dead code.

---

## 4. DEPENDENCY EDGES (from the DAG — ledger §4)

P-ACTUATION is a **WAVE-1 ROOT with ZERO inbound hard edges.** It authors the generic contract WITHOUT importing anything from arc (it promotes arc's *existing* enums into compute as new code).

| Edge | Type | Reason |
|------|------|--------|
| **P-ARC → P-ACTUATION** | **HARD (inbound)** | arc-as-instance `impl Actuation for ArcKnob` needs S2 trait + S1 refusal; AND P-ARC's shipped actuate path is DEAD until **S13** (this plan) lands. RESOLUTION-A sequences `arc_actuator.rs` behind P-ACTUATION (this plan goes FIRST). |
| **P-RECONCILE → P-ACTUATION** | **SOFT (inbound)** | P-RECONCILE's 3 `P2PCommand` lifecycle replies return `Result<(), ActuationRefusal>` (S1). Works standalone with a local refusal shim if this plan slips; the shim is deleted at integration. |
| mishpat zome → P-ACTUATION | **SOFT (inbound)** | the zome consumes `ScopeId::NetworkArc.scope_str()` (S3) to kill the duplicated literal. If `scope_vocab` can't be made wasm-safe, falls back to a shared const (Task 6 decides). |
| **P-ACTUATION → P-ARC** | **HAND-OFF (received, G0-C)** | receives `StaggerGate`/`may_arc_restart`; shim with `MetaCooldown` if absent. Not a compile dependency (the shim makes it standalone). |
| **P-ACTUATION → (none else)** | — | ZERO outbound hard edges. Roots: P-ACTUATION, P-RECONCILE, P-DEFENSE, P-PROOFS. |

**Cycle check (ledger §4):** the near-cycle P-ARC ↔ P-ACTUATION is BROKEN to strictly P-ARC → P-ACTUATION: this plan promotes arc's *existing* enums as new code (no dep on the arc crate), and P-ARC implements against it. No cycles.

---

## 5. TASK-BY-TASK (TDD — test first)

**Per-crate build/test discipline (memory-pinned, load-bearing):**
- `elohim-compute` is **native** → `RUSTFLAGS=""`.
- `elohim-storage` is the **WASM-flagged** crate → `RUSTFLAGS='--cfg getrandom_backend="custom"'` (the flag leaks → `undefined __getrandom_v03_custom` at link if you use `""`).
- the mishpat zome builds via its `justfile` (`just check`) — RUSTFLAGS is set there; do NOT override.
- ALWAYS `CARGO_TARGET_DIR=/tmp/<slot>` (pool fingerprint ENOENT) + `RUSTC_WRAPPER=""` (sccache spawn-ENOENT).
- **Plain `cargo test`, NEVER nextest** in this container. **Never `&&`-pipe a gate exit code** — use `2>&1 | tail -N`.

Task order: **T1 → T2 → T3** (compute primitives, no storage dep) then **T4 → T5 → T6** (storage + zome consumers, depend on T1-T3) then **T7** (http repoint, depends on T4). T1-T3 are independently parallelizable subagents; T4-T7 sequence on the shim.

---

### TASK 1 — `elohim_compute::actuation` (S1, S2, S4): the canonical contract

**p2p-class:** the contract types are pure Rust (no DHT entry, no table) — Cat-C node-local operational types. Cite the class; do not re-litigate.

Files: **C** `elohim/elohim-compute/src/actuation.rs`; **M** `elohim/elohim-compute/src/lib.rs` (append-only RESOLUTION-E block).

- [ ] Write the failing test — `elohim/elohim-compute/src/actuation.rs` (`#[cfg(test)] mod tests`):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_cooldown_admits_first_and_defers_within_window() {
        let cd = MetaCooldown::new(120);
        assert!(cd.admits(1_000).is_ok(), "first actuation always admits");
        let mut cd2 = MetaCooldown::new(120);
        cd2.mark(1_000);
        assert_eq!(cd2.admits(1_030), Err(90), "30s in: defer 90s");
        assert!(cd2.admits(1_120).is_ok(), "121s window elapsed: admit");
    }

    #[test]
    fn never_touch_default_breaches_nothing() {
        let nt = NeverTouch::default();
        let req = ActuationReq {
            scope: crate::scope_vocab::ScopeId::NetworkArc,
            value: 0,
            commitment_cid: "uhCkk".into(),
        };
        assert!(nt.breached_by(&req).is_none());
    }

    #[test]
    fn refusal_carries_code_and_elevate() {
        let r = ActuationRefusal {
            code: RefusalCode::GateRefused("coverage".into()),
            elevate: Some("add peers".into()),
        };
        assert_eq!(r.code, RefusalCode::GateRefused("coverage".into()));
        assert_eq!(r.elevate.as_deref(), Some("add peers"));
    }
}
```
- [ ] Run, expect FAIL: `cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo test --lib actuation 2>&1 | tail -40` — expect `cannot find … MetaCooldown` (and that `scope_vocab` is missing → land T2 first if so; the two compile together, author both before running).
- [ ] Write minimal implementation — the §3 S1/S2/S4 definitions (trait `Actuation`, `ActuationReq`, `GrantBounds`, `GateCtx`, `ActuationError`, `ActuationRefusal`, `RefusalCode`, `MetaCooldown`, `NeverTouch`). The trait has no default bodies. Append to `lib.rs` (after `:13 pub mod resources;`): `pub mod actuation;` and (after the `pub use` block) `pub use actuation::{Actuation, ActuationReq, ActuationRefusal, RefusalCode, GrantBounds, GateCtx, ActuationError, MetaCooldown, NeverTouch};`. (The `scope_vocab` re-export lands in T2's edit to the same append block.)
- [ ] Run, expect PASS (3 tests).
- [ ] Commit:
```
git add elohim/elohim-compute/src/actuation.rs elohim/elohim-compute/src/lib.rs
git commit -m "feat(elohim-compute): canonical Actuation contract (S1,S2,S4)

One refuse/authorize/gate/render shape for every dataplane knob (finding #5).
Promotes arc's ActuationRefusal/RefusalCode into compute (single owner);
adds the generic GateRefused variant so reconcile/transport don't re-invent.
MetaCooldown + NeverTouch are the DESIGN-7 rails as types. Modeled on the
WriteThroughState 4-layer override SHAPE; imports nothing from it.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### TASK 2 — `elohim_compute::scope_vocab` (S3): the scope namespace

**p2p-class:** pure enum + `&'static str` — Cat-C; no entity.

Files: **C** `elohim/elohim-compute/src/scope_vocab.rs`; **M** `elohim/elohim-compute/src/lib.rs` (same append block as T1).

- [ ] Write the failing test — `elohim/elohim-compute/src/scope_vocab.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arc_scope_str_is_the_canonical_knob_literal() {
        // This is the ONE definition of the string the mishpat zome validates
        // and the actuator renders — killing the duplicated literal.
        assert_eq!(ScopeId::NetworkArc.scope_str(), "conductor.target_arc_factor");
    }

    #[test]
    fn sweep_cadence_carries_its_name() {
        assert_eq!(ScopeId::SweepCadence("sweep.provide").scope_str(), "sweep.provide");
    }

    #[test]
    fn scope_strs_are_disjoint() {
        let all = [
            ScopeId::NetworkArc.scope_str(),
            ScopeId::DoorwayWarmupTimeout.scope_str(),
            ScopeId::StorageAdmission.scope_str(),
        ];
        let mut sorted = all;
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), all.len(), "scope strings must be unique");
    }
}
```
- [ ] Run, expect FAIL: `cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo test --lib scope_vocab 2>&1 | tail -40`.
- [ ] Write minimal implementation — the §3 S3 `ScopeId` enum + `scope_str()`. **Keep it `core`-only (no std imports)** so Task 6 can evaluate a wasm dep. Append to `lib.rs`: `pub mod scope_vocab;` + `pub use scope_vocab::ScopeId;`.
- [ ] Run, expect PASS (3 tests).
- [ ] Commit:
```
git add elohim/elohim-compute/src/scope_vocab.rs elohim/elohim-compute/src/lib.rs
git commit -m "feat(elohim-compute): ScopeId scope vocabulary (S3)

One home for actuatable knob/scope strings; kills the duplicated
\"conductor.target_arc_factor\" literal (zome commitments.rs:511 + actuator).
core-only so the wasm zome can evaluate it as a dep (Task 6 decides path).

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### TASK 3 — Final compute gate (lib compiles + clippy + fmt)

- [ ] `cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo test --lib 2>&1 | tail -40` (all actuation + scope_vocab tests green; no regression in existing compute tests).
- [ ] `cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo clippy -- -D warnings 2>&1 | tail -40`.
- [ ] `cd /projects/elohim/elohim/elohim-compute && cargo fmt --check`.
- [ ] No commit (gate only); if fmt rewrites, `git add` the formatted files and amend the T1/T2 commits or add a `style(elohim-compute)` commit.

---

### TASK 4 — Arc as the FIRST `Actuation` INSTANCE (`services/actuation/arc.rs`) + RESOLUTION-A shim

**This is the cardinal task: refactor the already-built bespoke `arc_actuator` so it becomes an INSTANCE of S2.** Model the runtime-mutation mechanism on `WriteThroughState`'s typed `Arc<RwLock>` poison-tolerant admin override.

**p2p-class:** Cat-C node-local operational instance (the decision core is pure; the apply shell touches fs/conductor). The actuation COMMAND it fulfills is **Cat-A notarized** (the `sets-authority-arc` `Mishpat::Commitment`, CID = entry_hash) — see §6. The instance does not create the commitment; it executes against an already-notarized grant.

Files: **C** `elohim/elohim-storage/src/services/actuation/mod.rs` + `arc.rs`; **M** `elohim/elohim-storage/src/services/arc_actuator.rs` (→ thin shim); **M** `elohim/elohim-storage/src/services/mod.rs` (`pub mod actuation;`).

**Refactor shape:**
1. `services/actuation/arc.rs` defines `struct ArcKnob` (the actuator) holding a **`Arc<RwLock<Option<ArcOverride>>>` live-override cell** mirroring `WriteThroughState`'s layer-4 (poison-tolerant `.unwrap_or_else(|p| p.into_inner())` on every read/write). For v1 the override cell holds an optional runtime `min_interval_s` for the stagger/cooldown (the runtime-mutable admin override the prompt asks the contract to model). The arc's decision core (`authorize`/`coverage_admits` as `gate`/`render_conductor_arc_factor` as `render`) is **moved** out of `arc_actuator.rs` into `impl Actuation for ArcKnob` (preserve the existing pure functions' bodies verbatim — they are already 17-test-proven; only re-wrap them as trait methods adapting `ArcGrantBounds`↔`GrantBounds` and `WouldBreakCoverage`→`GateRefused`).
2. `services/actuation/arc.rs` also houses the **`apply` shell** (moved from `arc_actuator::apply_arc_actuation`) — and this is the **G0-C receiver**: before `conductor.restart()`, call `conductor.may_arc_restart(now, min_interval)` (P-ARC's hand-off); if absent at integration, use the in-contract `MetaCooldown` (the override cell's interval) as the local stagger guard. Either path DEFERS on refusal (never forces).
3. `arc_actuator.rs` becomes a **thin re-export shim**: `pub use crate::services::actuation::arc::{ArcKnob, …};` plus a compat `pub use elohim_compute::actuation::{ActuationRefusal, RefusalCode};` AND a free `pub async fn apply_arc_actuation(...)` that delegates to the `ArcKnob` instance — preserving the EXACT signature `http.rs:2680` calls so the http call site is untouched in this task (http repoint is T7, optional tightening). The 17 existing unit tests move WITH the decision core into `arc.rs` (they test pure functions; relocate, do not rewrite).

> **Why preserve `apply_arc_actuation`'s signature in the shim:** RESOLUTION-A says reduce `arc_actuator.rs` to "a thin re-export shim (preserving the `http.rs` call site)." Keeping the free fn delegating to `ArcKnob` means T4 is non-breaking to `http.rs`; T7 then OPTIONALLY repoints `http.rs` to call the `ArcKnob` instance directly (cleaner) but the shim guarantees green even if T7 is deferred.

- [ ] **First**, `Read` `arc_actuator.rs` in full (done — see plan author's read) and `process_manager.rs` to confirm `ConductorManager`'s `restart()`/`config_path()` signatures the apply shell uses, and whether `may_arc_restart` exists yet (P-ARC may not have landed).
- [ ] Write the failing test — `services/actuation/arc.rs` (`#[cfg(test)] mod tests`), the contract-instance test (in addition to the relocated pure tests):
```rust
    #[test]
    fn arc_knob_is_an_actuation_instance() {
        use elohim_compute::actuation::{Actuation, ActuationReq, GrantBounds, GateCtx, MetaCooldown, NeverTouch};
        use elohim_compute::scope_vocab::ScopeId;
        let knob = ArcKnob::new();
        let req = ActuationReq { scope: ScopeId::NetworkArc, value: 0, commitment_cid: "uhCkk".into() };
        let bounds = GrantBounds { min: 0, max: 1, domain_floor: 3, expires_at_epoch_s: 0, scope: ScopeId::NetworkArc };
        // authorize: leecher (0) is within [0,1], no expiry → Ok.
        assert!(knob.authorize(&req, &bounds, 1_000).is_ok());
        // gate: N=14, floor=3 → remaining 13 >= 3 → admits.
        let ctx = GateCtx { now: 1_000, observed_n: 14, cooldown: MetaCooldown::new(0), never_touch: NeverTouch::default() };
        assert!(knob.gate(&req, &ctx).is_ok());
        // gate: N=2, floor=3 → remaining 1 < 3 → GateRefused.
        let thin = GateCtx { now: 1_000, observed_n: 2, cooldown: MetaCooldown::new(0), never_touch: NeverTouch::default() };
        match knob.gate(&req, &thin).unwrap_err().code {
            elohim_compute::actuation::RefusalCode::GateRefused(_) => {}
            other => panic!("expected GateRefused, got {other:?}"),
        }
    }

    #[test]
    fn arc_override_cell_is_poison_tolerant() {
        // Mirrors WriteThroughState's poison-tolerant layer-4 (do NOT harmonize
        // the divergent poison policies; this cell copies the pattern locally).
        let knob = ArcKnob::new();
        knob.set_min_interval(Some(std::time::Duration::from_secs(300)));
        assert_eq!(knob.min_interval(), Some(std::time::Duration::from_secs(300)));
        knob.set_min_interval(None);
        assert_eq!(knob.min_interval(), None);
    }
```
- [ ] Run, expect FAIL: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo test --lib actuation 2>&1 | tail -40` — `cannot find struct ArcKnob`.
- [ ] Write minimal implementation: create `services/actuation/mod.rs` (`pub mod arc;` + the tool-surface `enum ActuationTool` stub from §3 with all-but-arc returning `NotActuatable` + `// FOLLOW-ON` per deferred tool); create `arc.rs` with `ArcKnob` + `impl Actuation` (moved decision core, `ArcGrantBounds`↔`GrantBounds` adapter, `WouldBreakCoverage`→`GateRefused`) + the poison-tolerant override cell + the moved `apply` shell calling `may_arc_restart` (or `MetaCooldown` shim). Rewrite `arc_actuator.rs` to the thin shim (re-export + delegating `apply_arc_actuation`). Add `pub mod actuation;` to `services/mod.rs`.
- [ ] Run, expect PASS — both the new instance tests AND the relocated 17 pure tests: `... cargo test --lib actuation 2>&1 | tail -40` and `... cargo test --lib arc_actuator 2>&1 | tail -40` (shim re-exports keep the old path resolving).
- [ ] Commit:
```
git add elohim/elohim-storage/src/services/actuation/mod.rs elohim/elohim-storage/src/services/actuation/arc.rs elohim/elohim-storage/src/services/arc_actuator.rs elohim/elohim-storage/src/services/mod.rs
git commit -m "refactor(elohim-storage): arc becomes the first Actuation instance (S2, RESOLUTION-A)

Lifts the bespoke arc decision core into impl Actuation for ArcKnob in
services/actuation/arc.rs; runtime-mutable override cell modeled on
WriteThroughState's poison-tolerant Arc<RwLock> layer-4. arc_actuator.rs is
now a thin re-export shim preserving the http.rs call site. Receives P-ARC's
may_arc_restart before restart() (MetaCooldown shim if not yet landed, G0-C).

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### TASK 5 — S13: the `sets-authority-arc` projection arm (the dead-path fix)

**The single highest-leverage fix in the campaign.** Without it the shipped arc actuate path is dead: `sets-authority-arc` falls to `other =>` (`mishpat_projection.rs:180`) → `bounds_json: "{}"` + `state: "proposed"` → `http.rs:2641` (`if row.state != "active"`) rejects with 409 forever.

**p2p-class:** the projection RESULT is **Cat-C node-local** (the read-model row); the projected commitment itself is **Cat-A notarized** (the `Mishpat::Commitment`, CID = entry_hash). The arm parses the notarized payload into the operational row — no new entity, no new table (the `mishpat_commitments` table exists).

Files: **M** `elohim/elohim-storage/src/mishpat_projection.rs` (new `parse_sets_authority_arc` + a match arm before `other =>`).

**The `active` decision (load-bearing):** every other arm projects `state: "proposed"` (the memory pin `project_resilience_snapshot_humans_junction` notes "POST commitments inserts 'proposed' not 'active'"). But `handle_arc_policy_actuate` requires `state == "active"`. The `sets-authority-arc` grant, once notarized on the DHT (the zome validated it at create-time, `commitments.rs:481`) and projected, **IS the active authorization** — there is no separate accept step for an arc grant (unlike a two-party delegates-compute negotiation). Therefore the arm projects `state: "active"` directly. **Verify this is the intended lifecycle** by reading whether any accept/activate path exists for `sets-authority-arc` (grep `sets-authority-arc` across storage); if a separate activation exists, project `proposed` and document the activation seam instead. Record the chosen lifecycle in the commit body. (Recommended + expected: `active` — the grant is self-activating; the LIVE gates that protect against misuse are expiry + the coverage gate at actuation time, enforced by `ArcKnob`, not a projection-state gate.)

- [ ] Write the failing test — append to `mishpat_projection.rs` `#[cfg(test)] mod tests`:
```rust
    #[test]
    fn sets_authority_arc_projects_active_with_real_bounds() {
        let payload = serde_json::json!({
            "action": "sets-authority-arc",
            "scope": "agent:james",
            "provider": "agent:matthew-steward",
            "recipient": "agent:james",
            "bounds": { "knob": "conductor.target_arc_factor", "min_factor": 0, "max_factor": 1, "coverage_floor": 3 },
            "valid_from": "2026-06-14T00:00:00Z",
            "valid_until": "2099-12-31T00:00:00Z"
        });
        let proj = parse_commitment_payload(
            "sets-authority-arc",
            &payload.to_string(),
            "uhCkk-eh",
            "uhCkk-ah",
        ).unwrap();
        match proj {
            CommitmentProjection::Upsert(row) => {
                assert_eq!(row.action, "sets-authority-arc");
                assert_eq!(row.scope, "agent:james");
                assert_eq!(row.state, "active", "arc grants self-activate (no two-party accept)");
                assert_ne!(row.bounds_json, "{}", "bounds must be projected, not empty");
                // The bounds round-trip carries the factor range + coverage floor.
                let b: serde_json::Value = serde_json::from_str(&row.bounds_json).unwrap();
                assert_eq!(b["min_factor"], 0);
                assert_eq!(b["coverage_floor"], 3);
            }
        }
    }
```
- [ ] Run, expect FAIL: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo test --lib mishpat_projection 2>&1 | tail -40` — fails on `state == "active"` (the `other =>` arm projects `proposed` + empty bounds).
- [ ] Write minimal implementation — add a match arm BEFORE `other =>` in `parse_commitment_payload` (`:163-180`):
```rust
        "sets-authority-arc" => parse_sets_authority_arc(&payload, entry_hash, action_hash)
            .map(CommitmentProjection::Upsert),
```
  and the parser (mirror `parse_delegates_compute`'s fail-closed field extraction, but project `state: "active"` and round-trip the full `bounds` object verbatim):
```rust
/// Project a `sets-authority-arc` grant (the conductor authority-arc factor
/// grant; spec §5). The grant SELF-ACTIVATES — there is no two-party accept (a
/// node grants its OWN arc authority, or a steward grants a recipient's, and the
/// notarized DHT commitment IS the authorization). So we project state="active"
/// directly; the LIVE protections (expiry + the coverage gate) are enforced by
/// the ArcKnob actuator at actuation time, not by a projection-state gate. The
/// zome validator (commitments.rs:481) already structurally validated this shape
/// at create-time (knob == conductor.target_arc_factor, factors in {0,1},
/// coverage_floor > 0). Fail-closed on required fields. Cat-A notarized source;
/// Cat-C projected row.
fn parse_sets_authority_arc(
    payload: &serde_json::Value,
    entry_hash: &str,
    action_hash: &str,
) -> Result<NewMishpatCommitment, String> {
    let scope = payload.get("scope").and_then(|v| v.as_str())
        .ok_or_else(|| "sets-authority-arc payload missing 'scope'".to_string())?.to_string();
    let provider = payload.get("provider").and_then(|v| v.as_str())
        .ok_or_else(|| "sets-authority-arc payload missing 'provider'".to_string())?.to_string();
    let recipient = payload.get("recipient").and_then(|v| v.as_str())
        .ok_or_else(|| "sets-authority-arc payload missing 'recipient'".to_string())?.to_string();
    let valid_from = payload.get("valid_from").and_then(|v| v.as_str())
        .ok_or_else(|| "sets-authority-arc payload missing 'valid_from'".to_string())?.to_string();
    let valid_until = payload.get("valid_until").and_then(|v| v.as_str())
        .ok_or_else(|| "sets-authority-arc payload missing 'valid_until'".to_string())?.to_string();
    // Fail-closed: a notarized arc grant with absent bounds would let the
    // actuator parse_grant_bounds fail downstream; require the field here.
    let bounds_json = payload.get("bounds").map(|b| b.to_string())
        .ok_or_else(|| "sets-authority-arc payload missing 'bounds'".to_string())?;
    Ok(NewMishpatCommitment {
        cid: entry_hash.to_string(),
        action: "sets-authority-arc".to_string(),
        scope,
        provider,
        recipient,
        bounds_json,
        valid_from,
        valid_until,
        revoked_at: None,
        state: "active".to_string(),
        dht_anchor_hash: Some(action_hash.to_string()),
    })
}
```
- [ ] Run, expect PASS: `... cargo test --lib mishpat_projection 2>&1 | tail -40` (new test + the existing `other =>`/delegates-compute tests all green).
- [ ] Commit:
```
git add elohim/elohim-storage/src/mishpat_projection.rs
git commit -m "fix(elohim-storage): project sets-authority-arc with real bounds + active (S13)

The shipped arc actuate path was DEAD: sets-authority-arc fell to the
unknown-action arm -> empty bounds + state=proposed -> the actuate handler's
state==active gate rejected forever. Add parse_sets_authority_arc projecting
the full bounds and state=active (arc grants self-activate; expiry+coverage
are enforced live by the actuator). Single highest-leverage fix.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### TASK 6 — Kill the duplicated knob literal: zome + actuator consume `scope_vocab`

The string `"conductor.target_arc_factor"` is duplicated in the mishpat zome (`commitments.rs:511`) and the actuator. This task makes ONE definition authoritative.

**p2p-class:** no entity (string deduplication). The zome edit is COORDINATOR-side only → DNA-hash-NEUTRAL (the integrity zome is untouched; hot-swaps via `update_coordinators`, per the existing validator's doc-comment).

Files: **M** `elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs` (`:511`); (the storage-side actuator already routes through `arc.rs` which imports `ScopeId` from T4 — verify, no separate edit unless a literal survives).

- [ ] **Decide the wasm path FIRST** (the zome is `wasm32-unknown-unknown`): try adding `elohim-compute = { path = "…/elohim-compute", default-features = false }` to the zome's `Cargo.toml` and `just check`. `scope_vocab` is `core`-only (T2), so it SHOULD compile to wasm. If `elohim-compute`'s OTHER modules pull std/native deps that break the wasm build even with `default-features = false`, DO NOT force it — instead expose `scope_vocab` behind a cargo feature (`scope-vocab` default-on, gating the heavy modules off) OR fall back to a single shared `pub const ARC_KNOB: &str = "conductor.target_arc_factor"` in `scope_vocab.rs` that BOTH the zome (via the lightweight dep/feature) and storage import. The deduplication target is "one definition," not "one crate at any cost." Record the chosen path in the commit body.
- [ ] Write the failing/guard test — in the zome's `#[cfg(test)] mod tests` (sweettest-free unit), assert the validator accepts the canonical scope string sourced from `scope_vocab` and rejects a wrong one:
```rust
    #[test]
    fn arc_knob_string_is_sourced_from_scope_vocab() {
        // The validator's knob check must use the ONE canonical string.
        assert_eq!(
            elohim_compute::scope_vocab::ScopeId::NetworkArc.scope_str(),
            "conductor.target_arc_factor"
        );
    }
```
  (If the wasm dep path is rejected and the const-fallback is chosen, assert against the shared const instead.)
- [ ] Run, expect FAIL (dep not yet added): `cd /projects/elohim/elohim/holochain/dna/mishpat && just check 2>&1 | tail -40`.
- [ ] Write minimal implementation — replace the literal at `commitments.rs:511`:
```rust
    if bounds["knob"].as_str().unwrap_or("")
        != elohim_compute::scope_vocab::ScopeId::NetworkArc.scope_str()
    {
        return Err("bounds.knob must equal the canonical arc knob (scope_vocab::NetworkArc)".into());
    }
```
  (or the const form per the path decision). Add the dep/feature to the zome `Cargo.toml`.
- [ ] Run, expect PASS: `cd /projects/elohim/elohim/holochain/dna/mishpat && just check 2>&1 | tail -40`. **NOTE: `just check` is a type-check; the full DNA build/sweettest is the pre-push/CI backstop — do NOT run sweettest here (75-min cycle).** Also verify storage still compiles: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo build --lib 2>&1 | tail -20`.
- [ ] Commit:
```
git add elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs elohim/holochain/dna/mishpat/zomes/mishpat/Cargo.toml
git commit -m "refactor(mishpat): validate arc knob via scope_vocab (kills duplicated literal, S3)

bounds.knob now checks ScopeId::NetworkArc.scope_str() instead of a hardcoded
\"conductor.target_arc_factor\" duplicated across zome + actuator. COORDINATOR-
side only -> DNA-hash-NEUTRAL (update_coordinators hot-swap). Wasm path: <recorded>.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### TASK 7 — Repoint `http.rs` `handle_arc_policy_actuate` at the contract (RESOLUTION-F)

After T4's shim, `http.rs` already compiles (the shim preserves `apply_arc_actuation`). This task OPTIONALLY tightens the handler to call the `ArcKnob` instance + the canonical refusal directly, completing the repoint the ledger names.

**Coordination note (audit must-fix):** `http.rs:2764` has P-ARC's ONE additive `local_authored_bytes` line in `handle_arc_policy_status` — a DIFFERENT handler from `handle_arc_policy_actuate` (`:2590-2716`). Disjoint lines; no conflict. P-DIAGNOSTIC does NOT touch storage `http.rs` (RESOLUTION-F).

Files: **M** `elohim/elohim-storage/src/http.rs` (the `handle_arc_policy_actuate` import + the `use crate::services::arc_actuator::{self, ActuationRefusal, ApplyError}` at `:2594`, and the refusal destructure at `:2701`).

- [ ] `Read` `http.rs:2590-2716` (done). Confirm `:2594` imports + `:2680` apply call + `:2701` destructure (`ActuationRefusal { code, elevate }`).
- [ ] Update the import to source `ActuationRefusal`/`RefusalCode` from `elohim_compute::actuation` (or keep via the `arc_actuator` shim re-export — both resolve; prefer the canonical path so the shim can eventually be deleted). The `:2701` destructure on `elevate: Option<String>` now needs `elevate` printed with `.unwrap_or_default()` in the JSON (compat with the promoted `Option`). Keep `apply_arc_actuation` (the shim free fn) OR switch to `ArcKnob::default().apply(...)` — pick the smaller diff; document.
- [ ] Run, expect PASS: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo test --lib http 2>&1 | tail -40` (or `cargo build --lib` if no http unit tests cover this handler — it's exercised by integration, not unit).
- [ ] Commit:
```
git add elohim/elohim-storage/src/http.rs
git commit -m "refactor(elohim-storage): repoint arc actuate handler at the canonical contract (RESOLUTION-F)

handle_arc_policy_actuate now sources ActuationRefusal/RefusalCode from
elohim_compute::actuation (Option<String> elevate). Disjoint from P-ARC's
local_authored_bytes line in handle_arc_policy_status. Shim still preserves
the call site so the change is non-breaking.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## 6. P2P-CLASS OF THE ACTUATION COMMAND (p2p-design-gate — cite the class, do not re-litigate)

- **The actuation COMMAND is Cat-A notarized.** It is fulfilled against a `Mishpat::Commitment` with action `delegates-compute`/`sets-authority-arc` (the grant), CID = `entry_hash` (gospel `project_mishpat_commitment_cid_is_entry_hash` — the entry_hash is the bounds key the actuator fetches by; `action_hash` is only `dht_anchor_hash`). The DHT entry type already exists (S12, cad5fb67c); this plan creates NO new entry type and moves NO DNA hash (the zome edit in T6 is coordinator-side only — DNA-hash-NEUTRAL).
- **The actuation RESULT projects as Cat-C node-local.** The `sets-authority-arc` projection arm (S13) writes an operational `mishpat_commitments` row (read-model, not truth); the rendered conductor-config + the applied factor are node-local operational effects. No new table, no new signal, no new coordinator fn — the projection consumes the EXISTING `CommitmentCommitted` signal path.
- **Identity is content-derived** (CID = entry_hash); no slug, no agent-composite key invented.
- The contract types (`Actuation`, `GrantBounds`, `ActuationRefusal`, `ScopeId`, `MetaCooldown`, `NeverTouch`) are pure Cat-C operational types — no entity.

---

## 7. // FOLLOW-ON SEAMS (deliberately left for the integration pass)

- **`StaggerGate` call-wiring** — RECEIVED here (G0-C): `services/actuation/arc.rs` calls `conductor.may_arc_restart(...)` before `restart()`. If P-ARC's gate hasn't landed at integration, the in-contract `MetaCooldown` is the local stagger guard; the integrator swaps it for `may_arc_restart` (one site) when P-ARC lands. The gate must NOT be dead code.
- **The deferred tool surface** (`pause_sweep`/`resume_sweep`/`run_now`/`reset_connection`/`quarantine_peer`/`readmit_peer`/`adjust_flow_window`) — the SURFACE is defined (`ActuationTool` enum, T4); only `tune_knob` for `NetworkArc` is wired. `pause_sweep`/`resume_sweep`/`run_now` are OWNED by P-RECONCILE's `P2PCommand`s (this plan only reserves `ScopeId::SweepCadence`); the transport/peer tools are named follow-ons needing a transport/peer `impl Actuation`.
- **`rate_limit_rpm` as an `Actuation` instance** — P-DEFENSE DELETES the dead field; if the operator ever wants a real per-route limiter, it is an instance of THIS contract (S2), never a bespoke counter (integration plan §2.1, operator decision §5).
- **Runtime-tunable connection limits** — a `ScopeId::ConnectionFloor` variant + a transport `impl Actuation` + a P-RECONCILE `P2PCommand` — named follow-on, not built (P-TRANSPORT ships static config v1).
- **Fractional arc (option ii)** — REJECTED (P-ARC Decision Memo); arc's `derive` fractional aim stays a gauge SIGNAL; the contract's `value: u64` is `{0,1}` for `NetworkArc` until kitsune2 exposes a runtime tgt-arc API.
- **`arc_actuator.rs` shim deletion** — once every consumer imports the canonical paths (T7 done, no residual `arc_actuator::` references), the integration pass may delete the shim entirely (the CircuitBreaker single-owner discipline). Left as a shim this campaign for non-breaking landing.
- **`WriteThroughState` generalization** — explicitly NOT done. S10 stays write-through-specific; the contract only mirrors its shape.

---

## 8. DISPATCH NOTE

- **Isolated worktree**, subagent-driven (superpowers:subagent-driven-development). Do NOT run in the shared `feat` tree — P-RECONCILE (the other WAVE-1 root) and P-ARC mutate adjacent files; only `elohim-compute/src/lib.rs` overlaps another plan (RESOLUTION-E, append-only — mechanical merge).
- **Commit-only on the shift branch; the integrator pushes.** Never `git push`.
- **Per-task `git add` lists name exact files only** (selective-stage) — the worktree may carry ambient mods.
- **NO CODE is executed against a live cluster tonight — this is plan authoring; the build/test commands are for the dispatched worker.**
- **Runtime Rust must NEVER write `.claude/data`** (the elevate arm is an external poller) — the actuator only reads/writes conductor-config + the projection row.
- **WAVE-1 root — dispatch in parallel** with P-RECONCILE, P-DEFENSE, P-PROOFS-core. Gate to WAVE 2: P-ACTUATION's `Actuation`/`ScopeId`/`ActuationRefusal` + the S13 projection arm landed and merged so P-ARC (HARD) and P-RECONCILE (SOFT, deletes its shim) rebase onto a populated `elohim_compute::actuation`.
- **Sequencing within the plan:** T1→T2→T3 (compute, parallelizable subagents) before T4→T5→T6 (storage+zome consume the contract) before T7 (http tighten). T5 (S13) is independent of T1-T4 and may run in parallel — it touches only `mishpat_projection.rs`. T6's wasm-path decision is the only research step; if it dead-ends, the const-fallback keeps the deduplication.
