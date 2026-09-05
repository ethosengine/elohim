---
id: "backlog-projection-reconcile-loc-ceiling-decomposition"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Decompose p2p/projection_reconcile.rs — hard LoC-ceiling breach (finding c933fddb1025): split the four reconcile arms into sibling submodules by pure code motion"
slug: "projection-reconcile-loc-ceiling-decomposition"
written: "2026-08-09"
author: "rust-architect (source-file-loc-ceiling architecture finding c933fddb1025)"
status: "backlog"
priority: "medium"
finding: "c933fddb1025"
policy: "source-file-loc-ceiling@1"
relatedNodeIds:
  - "backlog-arch-dataplane-refactor-backlog"
  - "backlog-p2p-mod-loc-ceiling-decomposition"
  - "backlog-elohim-storage-p2p-modrs-modularization"
  - "backlog-arch-content-store-zome-modularization"
  - "backlog-doorway-http-rs-modularization"
  - "project_principle_p1_reconciliation_controller"
  - "feedback_signature_changes_grep_callers"
  - "feedback_swarm_composition_fresh_tree_build"
  - "feedback_concurrent_sessions_shared_worktree"
tags: [architecture, refactor, p2p, loc-ceiling, god-file, dataplane, reconcile, tech-debt, mod-decomposition]
cluster: "arch-dataplane-refactor-backlog#18"
cites:
  - elohim/elohim-storage/src/p2p/projection_reconcile.rs
  - .claude/epr-meta/policies.yaml
  - genesis/data/timeline/backlog/arch-dataplane-refactor-backlog.md
  - genesis/data/timeline/backlog/p2p-mod-loc-ceiling-decomposition.md
shift_objective: |
  Decompose elohim/elohim-storage/src/p2p/projection_reconcile.rs (7,339 lines at
  scoping; growing ~190 lines/commit under active drain-cure work) into sibling
  submodules under a NEW elohim/elohim-storage/src/p2p/projection_reconcile/
  directory, keeping projection_reconcile.rs itself as the module root at its
  CURRENT path depth. This is plain native Rust in elohim-storage — NOT a zome:
  no DNA hash moves, no reinstall, no coordinator hot-swap; the gate is
  fmt + clippy -D warnings + cargo test (nextest is NOT installed in this
  container) plus `cargo test export_bindings` with a sha256 diff of the
  generated ProjectionReconcileStatus.ts. Per wave: (1) move ONE cluster's items
  verbatim into a new submodule file; (2) `use`/`pub use` from the root so the
  module's external surface is byte-identical; (3) move that cluster's
  #[cfg(test)] tests with it; (4) run the gates. One cluster per commit so a
  regression bisects cleanly. DO NOT START until the in-flight drain-cure work
  on this file has landed on dev — a code-motion wave against a hot file is a
  merge-conflict generator. Ratchet loc-hard DOWN in .claude/epr-meta/policies.yaml
  as the root drains; never up.
---

# projection_reconcile.rs decomposition — four arms, one file

## Finding

`elohim/elohim-storage/src/p2p/projection_reconcile.rs` is **7,339 lines**
(worktree snapshot at scoping; the finding fired at ~7,344+ on a tree that was
mid-edit), past the `source-file-loc-ceiling@1` hard ceiling of **7,000**
(`.claude/epr-meta/policies.yaml`, `loc-soft: 3000` / `loc-hard: 7000`). The
policy's prescribed response is exactly this artifact — canonicalize a
modularization plan into the timeline backlog and drive it as bounded work;
**never refactor mid-edit**. No code was touched to produce this entry.

**Growth rate is the sharp part of the finding.** Across the last ten commits
touching this file: `4cfdd8e24` → 5,454 lines · `da752307f` → 6,474 ·
`ae76e67cd` (HEAD at scoping) → 7,339. That is roughly **+190 lines/commit**,
and it crossed the ceiling within the last three commits. The file is the live
notary-authority drain-cure surface, so it will keep growing until this work
lands. Line ranges below are therefore **approximate and already drifting** —
re-derive every wave from item names (`grep -n '^\(pub \)\?\(async \)\?fn \|^impl \|^pub struct \|^struct \|^pub enum \|^enum '`) and from the four `// ====` section banners, never from the numbers printed here.

## Refactor-safety class (READ FIRST)

Per the policy's `why`, three refactor classes carry very different risk. This
file sits in the **lowest** one.

| Class | Applies here? | Consequence |
|---|---|---|
| Integrity-zome change | **No** | Would move the DNA hash → reinstall / re-key / DHT-partition trap; ride a deliberate DNA-lineage event only |
| Coordinator-zome change | **No** | Would need the `update_coordinators` hot-swap path (`ALLOW_COORDINATOR_UPDATE`) |
| **Plain native Rust** | **Yes** | Gated by `cargo fmt` + `clippy -D warnings` + `cargo test` alone. No conductor involvement, no DNA hash, no deploy-order coupling. |

`projection_reconcile.rs` is a crate-internal module of the native
`elohim-storage` binary. It *calls* the conductor (`hc_client`,
`services::conductor_writes`) but defines no entry type, no `#[hdk_extern]`, no
validation callback. Moving code between modules inside this crate cannot move a
DNA hash and cannot partition a DHT. **Do not import the zome-decomposition
ceremony from `arch-content-store-zome-modularization` here** — no sweettest is
required, no extern-symbol baseline, no `ALLOW_*` flag.

### The one non-obvious risk: ts-rs

The file defines exactly one `#[derive(TS)]` type — `ProjectionReconcileStatus`
(with `#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]`),
which generates `elohim/sdk/storage-client-ts/src/generated/ProjectionReconcileStatus.ts`
and is contract-tested by `elohim/elohim-storage/tests/schema_contract.rs`.

Empirical evidence that **nesting depth is safe**: the identical `export_to`
string is used from two different source depths across two crates —
`elohim/elohim-views/src/acquisition.rs` (`src/<file>.rs`) and
`elohim/elohim-storage/src/p2p/acquisition.rs` (`src/<dir>/<file>.rs`) — so
`export_to` is not resolved relative to the source file's directory. A deeper
submodule therefore should not move the emitted `.ts`.

The plan nonetheless **keeps `ProjectionReconcileStatus` in the module root**
(belt and braces — the root file does not change path at all) and requires
`cargo test export_bindings` + `sha256sum` on the generated
`ProjectionReconcileStatus.ts` **before and after every wave**. If a wave ever
does move a ts-rs type and the sha changes, revert that wave — do not "fix" the
`export_to` string.

## The shape: keep the root file, add a sibling directory

Rust 2018 permits `foo.rs` and `foo/` to coexist. Use that.

```
src/p2p/projection_reconcile.rs        # module root — SAME path, SAME depth
src/p2p/projection_reconcile/          # NEW sibling directory
    ledger.rs
    pacing.rs
    circuit.rs
    routing.rs
    arm_rea.rs
    arm_witness.rs
    content_discover.rs
    content_heal.rs
    content_adopt.rs
    arm_collectives.rs
    arm_shard.rs
```

Why not `projection_reconcile/mod.rs`: keeping the root file in place means the
ts-rs-anchored type never changes path, the 95-line module doc keeps its
canonical home, and every `git blame` on the orchestrator stays intact. Nested
directory modules already exist in this crate (`graph_views/lamad`,
`p2p_iroh/dual_publish`), so the pattern is in-repo precedent, not an invention.

### The external surface that must stay byte-identical

The module's whole out-of-module API is four call sites. This is what makes the
decomposition cheap — it is a *pure internal* reorganization.

| Consumer | Imports |
|---|---|
| `src/main.rs` | `run_discovery`, `run_heal`, `heal_decision`, `HealAction`, `InventoryWindow`, `MissLedger`, `ProjectionReconcileState::new()` |
| `src/liveness_contract.rs` | `gapfill_would_self_elect`, `timeout_should_route_to_adopt`, `declared_divergence_should_route_to_contest`, `conductor_missing_should_route_to_adopt` |
| `src/p2p/mod.rs` | `ProjectionReconcileState`, `ProjectionReconcileStatus` |
| `tests/schema_contract.rs` | `ProjectionReconcileStatus` |

**Invariant**: after every wave, `use crate::p2p::projection_reconcile::{...}` in
those four files compiles **unchanged**. The mechanism is a `pub use` re-export
block in the root. Do not "tidy" a consumer's import path to point at a
submodule — that turns a code-motion wave into an API change and destroys the
bisect property.

## Structural map (what actually lives in there)

Four reconcile arms plus shared rails, in one file. Approximate line ranges from
the 7,339-line snapshot; **re-derive per wave**.

| Range | Cluster | ~LoC |
|---|---|---|
| 1–118 | Module doc (95 lines, four-arm contract) + imports | 118 |
| 120–325 | Witness caps, `Admission`, `advertised_head_corpus_digest`, `MissEntry`, `MissLedger` | 206 |
| 326–572 | Pacing constants, `HealPacing`, head-batch budget/resolver statics | 247 |
| 573–705 | `is_transient_conductor_error`, `is_synthetic_attempt_timeout`, `should_retry_attempt`, `HealCircuit`, `classify_reauthor_failure_class` | 133 |
| 706–934 | `HealOutcomeKind` + the four routing predicates | 229 |
| 935–998 | `RetryResult`, `call_with_retry` | 64 |
| 999–1157 | `ProjectionReconcileStatus` (ts-rs), `ProjectionReconcileState` | 159 |
| 1158–1217 | `ReaDiscovery` | 60 |
| 1218–1318 | `SweepPlan`, `HealAction`, `fold_arm_counts`, `heal_decision` | 101 |
| 1319–1488 | `InventoryWindow` + `inventory_window_tests` | 170 |
| 1489–1778 | `run_discovery` + `run_heal` (the orchestrator) | 290 |
| 1779–2152 | Witness arm: `witness_bootstrap`, `ContentHealOutcome`, `witness_ghost_anchors` | 374 |
| 2153–2581 | REA arm: `discover_rea`, `heal_rea`, `ReaHealOutcome`, `ReaAnchorWrite`, `classify_rea_anchor_write`, `heal_one` | 429 |
| 2582–4332 | **Content arm** (banner: *Content-anchor reconcile arm, notary-authority Leg 4*) | **1,751** |
| 4333–4996 | Collectives arm (banner) + `collectives_gap_tests` | 664 |
| 4997–5107 | Shard-location catch-up arm (banner) | 111 |
| 5108–7339 | `mod tests` | **2,232** |

Two facts worth naming:

1. **Tests are 30% of the file** (2,232 + 170 + 153 ≈ 2,555 lines across three
   `#[cfg(test)]` modules). Moving tests with their code is the single largest
   LoC lever and carries near-zero risk (`use super::*` becomes
   `use super::super::*` or explicit imports).
2. **The content arm alone (1,751 lines) is over the soft ceiling.** It gets
   three files, not one.

## Proposed decomposition

### Root keeps (target ≈ 700 lines)

The module doc (the four-arm design contract — it is the orientation document
for the whole subsystem and belongs at the root, with arm-specific paragraphs
migrating into each arm module's own `//!` header), `ProjectionReconcileStatus`,
`ProjectionReconcileState`, `SweepPlan`, `HealAction`, `fold_arm_counts`,
`heal_decision`, `run_discovery`, `run_heal`, the cross-arm convergence tests,
and the `pub use` re-export block.

`run_discovery` / `run_heal` are the arm scheduler — they own the leg-budget
ordering (REA → content → collectives, so collectives can never starve the two
before it) and are the natural root residents.

### Wave 1 — shared rails (leaf clusters, near-zero coupling)

| New module | Contents | ~LoC (incl. moved tests) |
|---|---|---|
| `ledger.rs` | `Admission`, `advertised_head_corpus_digest`, `MissEntry`, `MissLedger`, `InventoryWindow`, `inventory_window_tests`, the miss/witness caps (`MISS_READMIT_SWEEPS`, `MISS_LEDGER_CAP`, `MAX_INVENTORY_WINDOW_TOTAL`, `WITNESS_MAX_PER_TICK`, `WITNESS_SWEEP_BUDGET`) | ~600 |
| `pacing.rs` | `HealPacing` + `Default`/`test_fast` impls, the leg budgets (`REA_LEG_BUDGET`, `CONTENT_LEG_BUDGET`, `COLLECTIVES_LEG_BUDGET`), `HEAL_ATTEMPT_TIMEOUT`, backoff constants, `HEAD_BATCH_*` statics + `head_batch_budget_current` / `head_batch_budget_observe` / `head_batch_resolver`, `RetryResult`, `call_with_retry` | ~550 |
| `circuit.rs` | `is_transient_conductor_error`, `is_synthetic_attempt_timeout`, `should_retry_attempt`, `HealCircuit`, `classify_reauthor_failure_class`, `HEAL_SYNTHETIC_TIMEOUT_MARKER`, `HEAL_CIRCUIT_TIMEOUT_THRESHOLD` | ~330 |

These three are the highest-value / lowest-risk starting point: they are
constants + small pure functions + their tests, referenced by every arm, and
they drain ~1,480 lines with no async control flow moved at all.

**Constant-visibility caveat**: several of these constants are asserted directly
by tests (`the_attempt_timeout_stays_above_the_extern_budget`,
`witness_per_tick_cap_is_bounded_for_pacing`, `witness_sweep_budget_is_a_real_bound`,
`the_batch_fanout_is_lower_than_the_single_id_fanout_it_replaces`) and one is
name-referenced from `services/head_batch_resolver.rs` documentation. Constants
moved out of the root need `pub(crate)` (or `pub(super)`) visibility, not a
narrowing — a private-by-accident constant turns into a compile error in the
consuming test and tempts a test rewrite. **Widen visibility, never rewrite an
assertion, during a code-motion wave.**

### Wave 2 — routing predicates (the liveness-contract surface)

| New module | Contents | ~LoC |
|---|---|---|
| `routing.rs` | `HealOutcomeKind` + its impl, `gapfill_would_self_elect`, `timeout_should_route_to_adopt`, `declared_divergence_should_route_to_contest`, `conductor_missing_should_route_to_adopt`, and their tests | ~600 |

Handle this wave with extra care for one reason: these four predicates are the
**only** items `liveness_contract.rs` imports, and `liveness_contract.rs` runs
them through the `seam_contracts::harness::liveness` state-machine harness. The
`pub use` re-export must keep the path `crate::p2p::projection_reconcile::<fn>`
valid verbatim. `HealOutcomeKind`'s label set is also asserted stable
(`heal_outcome_labels_are_stable`) and referenced from `metrics.rs` docs (30
metric series) — the label strings are a wire-ish surface; move them, never
touch them.

### Wave 3 — the small arms

| New module | Contents | ~LoC |
|---|---|---|
| `arm_rea.rs` | `ReaDiscovery`, `discover_rea`, `heal_rea`, `ReaHealOutcome`, `ReaAnchorWrite`, `classify_rea_anchor_write`, `heal_one`, + the REA tests | ~700 |
| `arm_shard.rs` | `reconcile_shard_locations_from_peers` (the Category-C custody catch-up arm) | ~111 |
| `arm_witness.rs` | `witness_bootstrap`, `witness_ghost_anchors`, + witness-classifier tests | ~600 |

**`arm_shard.rs` carries an open question, not a code-motion decision.**
`reconcile_shard_locations_from_peers` is `pub async fn` with **no caller
anywhere in the repo** (verified by repo-wide grep: the only hit is its own
definition). It is either dead, or a wired-elsewhere-later arm, or a wiring
regression. **The code-motion wave must move it as-is and change nothing.**
Raise the wiring question as a separate finding — deleting or "fixing" a dead
`pub` inside a refactor wave is exactly how a pure-motion commit stops being
bisectable.

`arm_witness.rs` has one entanglement: `ContentHealOutcome` is *defined* between
the two witness functions but is the **content arm's** heal-outcome type
(constructed in `heal_content`, destructured in `run_heal`, mirrored by
`ReaHealOutcome`). It belongs in `content_heal.rs`, not `arm_witness.rs` — the
current adjacency is an artifact of append-order, not of design. Move it with
the content heal wave and let `arm_witness.rs` import it.

### Wave 4 — the content arm (largest; three files)

| New module | Contents | ~LoC |
|---|---|---|
| `content_discover.rs` | `ContentDiscovery`, `ContentGap`, `classify_content_gap`, `discover_content`, + gap-classification tests | ~650 |
| `content_heal.rs` | `ContentHealOutcome`, `resolve_pipeline`, `apply_replayed_missing`, `heal_content`, `heal_content_one`, + the DRAIN LEVERS test section | ~900 |
| `content_adopt.rs` | `AdoptCandidate`, `compose_adopt_slice`, `adopt_deferred_heads`, `batch_probe_elections`, + the tail-reservation tests | ~700 |

This arm carries the heaviest external coupling in the file — 41 references to
`crate::services::head_adoption`, plus `head_batch_resolver`, `contest_backoff`,
`heal_backoff`, `reanchor_backfill`, `content_diesel`. That coupling is a
*reason to split it*, not a reason to defer it: three files each with a coherent
dependency fan is materially more reviewable than one 1,751-line block.

`compose_adopt_slice` is doc-referenced from `liveness_contract.rs` (as the
scheduler seam) and `services/head_adoption.rs` — check whether either
reference is a live `use` at wave time (at scoping, both are doc-link only) and
re-export accordingly.

### Wave 5 — collectives

| New module | Contents | ~LoC |
|---|---|---|
| `arm_collectives.rs` | `CollectivesDiscovery`, `CollectiveGap`, `classify_collective_gap`, `discover_collectives`, `CollectivesHealOutcome`, `heal_collectives`, `heal_collective_one`, `collectives_gap_tests` | ~700 |

Self-contained (its own `GapTracker`, its own `HealPacing` budget, ordered last
in the sweep, `h_app_id="lamad"` on both read and write). It also has the
richest arm-specific `//!`-doc material in the root header — the three
load-bearing decisions (NULL-`collective_cid` exclusion, own tracker/budget
ordered last, cid-is-identity/`id`-is-routing-alias). Migrate those three
paragraphs into `arm_collectives.rs`'s own module header and leave a one-line
pointer in the root doc.

### Projected result

| | Before | After |
|---|---|---|
| `projection_reconcile.rs` | 7,339 | ~700 |
| Largest submodule | — | ~900 (`content_heal.rs`) |
| Files over `loc-soft` (3,000) | 1 | 0 |
| Files over `loc-hard` (7,000) | 1 | 0 |

## Gates (per wave)

Plain native Rust — the lowest-ceremony class. From
`elohim/elohim-storage/`, with the pool target slot set (`cargo-pool key`) and
the getrandom backend flag kept (it is a dependency requirement for this crate,
not a WASM build):

```
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
cargo fmt --check
cargo clippy -- -D warnings
cargo test                      # FULL crate lib test — nextest is NOT installed in this container
cargo test export_bindings      # then sha256sum elohim/sdk/storage-client-ts/src/generated/ProjectionReconcileStatus.ts
```

Discipline notes that bite this specific file:

- **Never pipe a gate's output.** `cargo test | tee` masks the exit code; a red
  run reads as green. Echo `EXIT=$?` on its own line.
- **Serialize your own cargo runs** against the shared target slot; assume
  parallel agents exist. Concurrent cargo corrupts a shared target dir.
- **Warm-slot illusion.** A warm target slot can make a wave read green for a
  tree that no longer builds cold. At minimum, build cold once at the end of the
  wave sequence.
- **`cargo build --workspace`** at the end of the sequence — per-crate green is
  not workspace green, and `liveness_contract.rs` + `tests/schema_contract.rs`
  are the cross-file consumers most likely to catch a re-export miss.
- **`rg` the moved item names crate-wide** (including `tests/`) after each wave.
  Signature-change caller sweeps are the #1 cause of pre-push failures 30+
  minutes after the edit; the same applies to visibility changes.
- **Stage only your own paths.** This tree is shared and carries concurrent
  operator/agent work: `git add <explicit paths>`, never `-A`. If `cargo fmt`
  reformats committed-clean files beyond your change, leave those unstaged.

## Readiness notes

- **BLOCKED on the in-flight drain-cure work.** At scoping (2026-08-09) another
  session was mid-edit on this exact file implementing a batch of drain-cure
  fixes, and the file has been growing ~190 lines/commit for ten commits. A
  code-motion wave against a hot file is a merge-conflict generator with an
  unusually bad payoff ratio — the conflicts are 100-line block moves, which git
  resolves worst. **Precondition: the drain-cure batch has landed on `dev` and
  the file has been quiet for at least one integration cycle.** Confirm with
  `git log -3 -- elohim/elohim-storage/src/p2p/projection_reconcile.rs` before
  starting Wave 1.
- **Waves 1–3 are independent** of each other and of 4–5; Wave 4 (content) is
  the highest-coupling and should not go first. Wave 2 (routing) touches the
  only external-consumer surface besides the orchestrator, so run it when you
  can afford a careful `liveness_contract.rs` re-check.
- **One cluster per commit.** Do not merge waves. The whole value of pure code
  motion is that a behavioral regression bisects to one move.
- **Zero behavior change is the contract.** No signature changes, no renames, no
  clippy-suggested "improvements" bundled in. Visibility widening
  (`pub(crate)` / `pub(super)`) is the ONLY permitted edit beyond the move and
  the `use`/`pub use` plumbing. If clippy fires a *new* warning on moved code,
  the fix is a scoped `#[allow]` with a follow-up finding — not a rewrite inside
  the motion commit.
- **Tests move with their code.** The 2,232-line `mod tests` is not one unit —
  map each test cluster to the arm it exercises. Cross-arm tests
  (`fold_arm_counts` convergence, `every_heal_arm_ends_with_update_caught_up`,
  the status-serialization tests) stay in the root. Note that
  `every_heal_arm_ends_with_update_caught_up` **scans the source text of the arm
  outcome-struct constructions** — verify how it locates them before moving arm
  code, or it will silently stop covering what it was written to cover. That
  test is a structural assertion about file layout and is the single most
  likely thing to break invisibly during this refactor.
- **Ceiling ratchet.** As the root drains below each threshold, ratchet
  `loc-hard` (and eventually `loc-soft`) **down** in
  `.claude/epr-meta/policies.yaml` (`source-file-loc-ceiling`) — never up. This
  is the policy's own instruction, and it is what converts a one-time cleanup
  into a held boundary.
- **Cluster row.** This plan is row **17** of
  `arch-dataplane-refactor-backlog` (dataplane *internal* reshaping). The row
  carries the ranked one-liner and the sequencing constraint; this file carries
  the wave plan. Groom them together.
- **Sibling precedent.** `p2p-mod-loc-ceiling-decomposition` (the 7.8k-line
  `p2p/mod.rs`, same crate, same policy, same pure-code-motion method) is the
  closest analogue — read its Phase 1 shape before starting. Coordinating the
  two matters: they are *different files*, so the waves are concurrency-safe
  except for `p2p/mod.rs`'s `pub mod` block — land them in separate commits.
  The cluster's rows 10 → 12 → 15 are the `p2p/mod.rs` chain and carry the same
  caveat.
- **No Dockerfile / manifest impact.** No new crate, no new path-dep, no new
  `[[bin]]`/`[[bench]]` target — the Dockerfile target-completeness and
  path-dep-COPY rules do not apply. No `Cargo.toml` change of any kind.

## Definition of done

Every wave: green `cargo build --release`, `cargo fmt --check`,
`cargo clippy -- -D warnings`, full-crate `cargo test`, and a byte-identical
`ProjectionReconcileStatus.ts` (sha256). Whole objective:
`projection_reconcile.rs` under the ratcheted `loc-hard` ceiling with no
submodule over `loc-soft`; the four external consumers
(`main.rs`, `liveness_contract.rs`, `p2p/mod.rs`, `tests/schema_contract.rs`)
compiling with **unchanged import statements**; zero behavior diff; and the
ceiling lowered in `.claude/epr-meta/policies.yaml` to lock the gain.
