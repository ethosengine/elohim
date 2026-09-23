---
id: "backlog-metrics-rs-loc-ceiling-decomposition"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Decompose elohim-storage/src/metrics.rs — hard LoC-ceiling breach (finding fe9edbbb11cd): one concern per submodule, each owning its statics, registration, pre-touch, setters and tests"
slug: "metrics-rs-loc-ceiling-decomposition"
written: "2026-09-22"
author: "rust-architect (source-file-loc-ceiling architecture finding fe9edbbb11cd)"
status: "backlog"
priority: "medium"
finding: "fe9edbbb11cd"
policy: "source-file-loc-ceiling@1"
relatedNodeIds:
  - "backlog-arch-dataplane-refactor-backlog"
  - "backlog-projection-reconcile-loc-ceiling-decomposition"
  - "backlog-p2p-mod-loc-ceiling-decomposition"
  - "backlog-no-latency-metric-for-change-doorbell"
  - "backlog-transport-route-metrics-pretouch-zero"
  - "backlog-deprecation-prometheus-014-proto-ext-legacy-getters"
  - "runtime-performance"
  - "feedback_signature_changes_grep_callers"
  - "feedback_concurrent_sessions_shared_worktree"
  - "feedback_subagent_silent_impl_drops"
tags: [architecture, refactor, loc-ceiling, god-file, observability, prometheus, tech-debt, mod-decomposition, constraint]
cluster: "arch-dataplane-refactor-backlog#20"
cites:
  - elohim/elohim-storage/src/metrics.rs
  - .claude/epr-meta/policies.yaml
  - genesis/data/timeline/backlog/arch-dataplane-refactor-backlog.md
  - genesis/data/timeline/backlog/projection-reconcile-loc-ceiling-decomposition.md
shift_objective: |
  Decompose elohim/elohim-storage/src/metrics.rs (6,797 lines on dev at scoping;
  7,018 and 7,106 on two live sprint worktrees) into concern submodules under a
  NEW elohim/elohim-storage/src/metrics/ directory, keeping metrics.rs as the
  module root at its CURRENT path. Plain native Rust: no zome, no DNA hash, no
  conductor, no ts-rs type in the file. Gate per wave = cargo fmt + clippy
  -D warnings + full-crate `cargo test --lib --bins` + the exposition-skeleton
  diff (Wave 0) + the public-item count guard. Waves 1–7 are pure code motion,
  one concern per commit, each concern moving its lazy_static statics, its typed
  setters and label enums, and its #[cfg(test)] tests together; `pub use` from
  the root keeps every `crate::metrics::X` / `elohim_storage::metrics::X` path
  compiling unchanged. Wave 8 (the only non-motion wave) replaces the monolithic
  register_all body with per-module `register(&Registry)` fns called under the
  same Once. DO NOT START until the three in-flight commits that edit metrics.rs
  (19c78d4af conductor cell state, fef7f7702 projected-apply staleness, ef752ee26
  pointer audit 1.4d; carried by ~20 sprint worktrees) are on dev. Then run Wave 0
  + Wave 1 at once and announce the module layout rather than waiting for a quiet
  window that the worktree fan-out will not produce. Ratchet loc-hard DOWN in
  .claude/epr-meta/policies.yaml afterward (a new policy version); never up.
---

# metrics.rs decomposition: 149 metric statics, one file, four places per metric

## Finding

Finding `fe9edbbb11cd` reports that `elohim/elohim-storage/src/metrics.rs` is at or over the
`source-file-loc-ceiling@1` hard ceiling of **7,000** lines (`.claude/epr-meta/policies.yaml`:
`loc-soft: 3000`, `loc-hard: 7000`). The policy's prescribed response is this artifact: a
modularization plan canonicalized in the timeline backlog and driven as bounded work. **No code
was touched to produce this entry.**

**The measure, verified against disk (2026-09-22):**

| Tree | Lines |
|---|---|
| `dev` HEAD `7afd76638` (also the main checkout) | 6,797 |
| `.worktrees/doorbell-latency-20260922` | 7,018 |
| `.worktrees/pointer-audit-1.4d` | 7,106 |

The finding's "7039+" came from a sprint tree. `dev` is just under the ceiling, but the file
will cross it when either branch lands. Growth is steep: 6,033 lines (`cf280f8fd`, 2026-09-17)
→ 6,797 lines (2026-09-22) in five days, with +391 lines in one commit (`7afd76638`,
runtime-performance instruments). 104 commits have touched the file since it was created on
2026-06-17. Every new instrument in the crate lands here, so the file grows as long as
instrumentation work continues.

**Re-measured 2026-09-23 (second dispatch of the same finding; plan re-verified, not re-derived).**
`dev` is still 6,797 lines at `71d95bd9b`, and the item counts under Invariant 2 still match
(149 / 122 / 15 / 11 / 2 / 15). The finding now reaches the harvester from the
`pointer-audit-1.4d` and `blob-put-off-path` worktree ledgers. It is not yet in the `dev`
ledger, because `dev` has not crossed. The growth sits in three commits that are not on `dev`
yet. About twenty sprint worktrees carry some of them:

| Commit | Adds | Lines | Lands in module |
|---|---|---|---|
| `19c78d4af` conductor running-cell vs app-enablement | `CONDUCTOR_CELL_RUNNING`, `CONDUCTOR_CELL_STATE`, `CONDUCTOR_SUPERSEDED_OBSERVATIONS` + 4 setters | +119 | `conductor.rs` |
| `fef7f7702` remote serving projection staleness | `SYNC_PROJECTED_APPLY_STALENESS_SECONDS` / `_INVALID` + observe fns + 1 test | +102 | `sync_plane.rs` |
| `ef752ee26` torn declared-row blob-pointer heal (1.4d) | `CONTENT_POINTER_AUDIT` + `PointerAuditOutcome` + `inc_pointer_audit` | +88 | `content_head.rs` |

Measured trees: `dev` 6,797; 7,018 in about 12 worktrees (the first two commits); 7,106 in
`pointer-audit-1.4d`, `pa-ship`, `rtt-ship` and `storage-batch` (all three); 7,041 in
`blob-put-off-path` (the first two commits plus +23 uncommitted). The `dev` file will cross
7,000 the moment the first two commits land. Because the work is spread across that many
worktrees, a "quiet for one integration cycle" window will not come on its own. The Readiness
section now gates on these three commits landing, not on named branches. Each of the three
instruments belongs to a concern module in the layout below, so it moves with that concern in Wave 4 (`sync_plane.rs`), Wave 5 (`conductor.rs`) or
Wave 7 (`content_head.rs`). None of them needs a new module.

**Line ranges below will drift.** Re-derive them each wave from item names
(`grep -n 'pub static ref\|^pub fn\|^pub(crate) fn\|^pub enum\|^pub const\|^pub struct'`) and
from the `// ──` section banners. Do not use the numbers printed here.

## Refactor-safety class (read this first)

The policy `why` separates three refactor classes. This file is in the **lowest-risk** one.

| Class | Applies? | Consequence |
|---|---|---|
| Integrity-zome change | **No** | — |
| Coordinator-zome change | **No** | — |
| **Plain native Rust** | **Yes** | Gated by fmt + clippy + tests alone. No DNA hash, no hot-swap, no deploy ordering. |

`metrics.rs` is a crate-internal module of the `elohim-storage` binary (`src/lib.rs:123`,
`pub mod metrics`). It contains **no `#[derive(TS)]` type**, so there is no `export_bindings`
sha step. It is not a zome, so no sweettest is needed.

### What is load-bearing: the exposition is a wire contract

Metric family names, label keys, and the **pre-touched zero series** are read by the Prometheus
PodMonitor, by dashboards and alerts, by a2o receipts, and by the habit checks
(`runtime-performance`, `dataplane-convergence`, `sync-scale-honesty`). The pre-touched series
matter because they separate a *measured zero* from an *absent* series. Examples:
`TRUST_PRICED_ADOPTIONS`, all eight `ConvergenceAtom` labels, and the
`NODE_DB_READ_POOL_*` kinds. A move that drops a single pre-touch will not fail compilation. It
makes a fleet series vanish, and that reads as "never deployed".

Two facts make the move safe to verify:

1. **Exposition order does not depend on registration order.** In prometheus 0.14,
   `Registry::gather` collects families into a `BTreeMap` keyed by name and sorts the metrics
   inside each family (`prometheus-0.14.0/src/registry.rs:119-184`). Moving statics between
   modules, or reordering `register` calls, cannot change `gather_text()` output.
2. **The external surface is path-only.** 67 files reference `metrics::`. That includes about
   30 statics read directly (such as `PROJECTION_REFUSED_STALE_REASONS`,
   `CONDUCTOR_CALL_DURATION_MS`, `ELOHIM_CUSTODIAN_USED_BYTES`), label enums (`ContestSkip`,
   `AdoptEvidence`, …), and constants (`PRIVATE_WITHHOLD_SITE_*`, `MINTED_SOURCE_*`). The
   package binary `src/main.rs` (it calls `register_all` and reads statics) and three
   integration tests import `elohim_storage::metrics` (`tests/rea/binding_attribution_cut.rs`,
   `tests/iroh_planes/iroh_sync_announce.rs`, `tests/iroh_planes/iroh_sync_driver.rs`). Glob
   re-exports from the root preserve all of these paths.

## The god-file mechanism: one metric lives in four places

Adding one metric today means editing four separate places:

1. a `pub static ref` inside one of **five** `lazy_static!` blocks (lines ~24–2575). The block
   was split five times *only* to stay under `__lazy_static_internal!`'s macro-recursion limit.
   The comments at the split points say so.
2. a `REGISTRY.register(...)` line plus any pre-touch loop in `register_all` (~2590–3128, about
   540 lines).
3. a typed setter and, often, a `seam_contracts::ReasonLabel` label enum (~3130–5280; 122 pub
   fns, 15 enums, 15 `ReasonLabel` impls).
4. a test in the single `mod tests` (~5282–6797, 1,515 lines, 37 `#[test]` fns plus 2 helpers).

These places sit ~2,500 lines apart. This is the "two-location sync trap" the policy names,
here with four locations. Forgetting step 2 compiles cleanly and then silently exports nothing.
Grouping the four places by concern inside one small module removes the trap. That makes
Wave 8 the main payoff of this plan, not an optional clean-up.

## Shape: keep the root file, add a sibling directory

This follows the `projection_reconcile.rs` + `projection_reconcile/` precedent (Rust 2018
allows `foo.rs` and `foo/` side by side):

```
src/metrics.rs              # root: module doc, REGISTRY, PROCESS_START_TIME_SECONDS,
                            #   register_all (Once), gather_text, `pub use` block,
                            #   cross-concern integration test, test_support lock
src/metrics/
    node_resources.rs       # proc RSS/threads, cgroup, DB readers + read-pool saturation,
                            #   CPU quota, conductor smaps/anon buckets, corpus docs
    conductor.rs            # admission (capacity/in-flight/wait/hold/acquired/shed),
                            #   conductor calls + ConductorCallMetricsGuard, chain-write
                            #   serialization/head-moved, head-plane BATCH externs,
                            #   ConvergenceAtom + ATOM_DURATION_MS, DB diagnostic query,
                            #   CONDUCTOR_APP_ENABLED
    http.rs                 # HTTP_REQUEST_*, classify_http_route, status_class,
                            #   RequestMetricsGuard
    sync_plane.rs           # SYNC_*, IROH_*, inventory pages, content touches,
                            #   view-federation, HEAD_ADOPTION_TRIGGER, blob swarm
    acquisition.rs          # ACQUISITION_*, TRANSPORT_ROUTE / PATH_RTT,
                            #   AcquisitionReconcileOutcome
    projection_reconcile.rs # PROJECTION_*, CONVERGED_BLOCKED_BY_TERMS, converged_*,
                            #   record_reconcile_sweep, shard-redistribute, peer-status fan-in
    content_head.rs         # head record/degraded/fetch, ghost decay, witness authored/
                            #   reauthor/sweep, canonical answers + links minted,
                            #   election obey/probe, release adoption decisions
    content_adopt.rs        # contest failed/skipped/backoff/remint, adopt evidence (+fallback),
                            #   adopt sweep, trust-priced adoption, reanchor, REA-heal skips
    custody.rs              # placement gap, RS coverage, custodian bytes, custody class counts,
                            #   projections shaded, rotation, announce, provider-unresolved,
                            #   shard push
    identity.rs             # namespace violations, key supersede, identity fill,
                            #   attribution joins/bindings, app deliverability,
                            #   account-caller unresolved, signal decode miss,
                            #   private withheld / preauth skipped
```

Each concern file holds its own `lazy_static!` block. Every block is then well under the
recursion limit, which retires the "separate block, purely for the recursion limit" comments
that account for the current five-block split.

### Approximate size per module (items + doc comments, then tests)

| Module | Items (~LoC) | Tests (~LoC) | Total (~LoC) |
|---|---|---|---|
| root `metrics.rs` (end state after Wave 8) | ~90 | ~330 (`register_all_idempotent_and_gathers_all_metrics`, process-start) | **~420** |
| `content_head.rs` | ~700 | ~150 | ~850 |
| `content_adopt.rs` | ~650 | ~70 | ~720 |
| `sync_plane.rs` | ~600 | ~50 | ~650 |
| `conductor.rs` | ~710 | ~390 | ~1,100 |
| `projection_reconcile.rs` | ~710 | ~345 | ~1,050 |
| `identity.rs` | ~340 | ~50 | ~390 |
| `custody.rs` | ~320 | ~40 | ~360 |
| `acquisition.rs` | ~270 | ~50 | ~320 |
| `node_resources.rs` | ~260 | ~10 | ~270 |
| `http.rs` | ~140 | ~200 | ~340 |

Until Wave 8 runs, each module's `REGISTRY.register` lines stay in the root's `register_all`
(about 540 lines). The largest module ends near 1,100 lines, well under the 3,000 soft ceiling.

## Invariants every wave must hold

1. **Paths are unchanged.** Every existing `crate::metrics::X` and `elohim_storage::metrics::X`
   compiles without edits. The root carries `pub use self::{node_resources::*, conductor::*, …};`
   plus a separate `pub(crate) use` for the two `pub(crate)` fns (`observe_db_diagnostic_query`,
   `observe_db_read_pool_saturation`), because a `pub` glob does not re-export crate-visible
   items. Do **not** retarget any consumer import to a submodule path. That would turn code
   motion into an API change and lose clean bisection.
2. **Silent-drop guard.** Before and after each wave, count these items across `metrics.rs` plus
   `metrics/*.rs`: `pub static ref`, `^pub fn|^pub(crate) fn`, `^pub enum`, `^pub const`,
   `^pub struct`, and `impl seam_contracts::ReasonLabel for`. At scoping the counts were
   149 / 122 / 15 / 11 / 2 / 15 (re-take them at Wave 0). The totals must match exactly.
   See [[feedback_subagent_silent_impl_drops]].
3. **The exposition skeleton is byte-identical** (Wave 0 harness).
4. **Shared test lock.** `SWEEP_GAUGE_LOCK` is taken by the projection sweep test
   (`the_actionable_count_gauge_and_term_flag_publish_from_one_fold`) *and* by the root
   integration test (`register_all_idempotent_and_gathers_all_metrics`). The two neighbouring
   converged-* tests call pure fns and do not need it. Move it to a `#[cfg(test)] pub(crate) mod test_support` in the root.
   Every test that takes it must keep taking it. Splitting it into per-module locks brings back
   the set-then-assert race its comment describes.
5. **One concern per commit**, with its tests. Stage only explicit paths. Never stage the
   operator's in-flight files. See [[feedback_concurrent_sessions_shared_worktree]].

## Waves

| Wave | Content | Class |
|---|---|---|
| **0** | Precondition: no live branch edits `metrics.rs` (check `.worktrees/*` and sprint branches with `git log dev..<branch> -- elohim/elohim-storage/src/metrics.rs`). Take the item-count baseline. Add a test `dump_exposition_skeleton` that, only when `METRICS_SKELETON_OUT` is set in the *caller's* environment (no test ever `set_var`s it), calls `register_all()` and writes each `# HELP`/`# TYPE` line and each series line with its value stripped to that path. Run it alone with `cargo test --lib metrics::tests::dump_exposition_skeleton -- --exact` in a fresh process, so only `register_all` pre-touches are present. Save the output as the baseline. | test-only |
| 1 | `http.rs` (smallest and self-contained; proves the glob-re-export and `use super::*` test pattern) | motion |
| 2 | `node_resources.rs` | motion |
| 3 | `identity.rs` + `custody.rs` | motion |
| 4 | `acquisition.rs` + `sync_plane.rs` | motion |
| 5 | `conductor.rs` | motion |
| 6 | `projection_reconcile.rs`, plus the `test_support` lock move | motion |
| 7 | `content_head.rs` + `content_adopt.rs` | motion |
| **8** | Each module gains `pub(super) fn register(r: &Registry)` holding its `register` lines and pre-touch loops, cut verbatim from `register_all`. `register_all` becomes the `Once` wrapper plus about 11 calls. **Not pure motion.** The skeleton diff is the proof that no registration or pre-touch was lost. | restructure |
| 9 | Ratchet: a new `source-file-loc-ceiling` policy version, or record the drained file against the existing one. Update any doc that cites a moved test path. Current citers: `deprecation-prometheus-014-proto-ext-legacy-getters.md:126` (`metrics::tests::app_deliverability_verdict`) and `2026-08-10-adam-pull-loop-wedged-at-boot.md:245` (`metrics::tests::acquisition_reconcile_outcomes_…`). Re-grep with `rg 'metrics::tests::'` first. Also add a module-doc paragraph telling authors where a new instrument goes: its concern module, defined, registered, setter and test side by side. | docs/policy |

**Gate per wave**, run in this repo's pool slot with the crate's `RUSTFLAGS` (prefer
`just gate elohim-storage`): `cargo fmt`, `clippy -D warnings`, `cargo test --lib --bins`
(nextest is not installed; echo `EXIT=$?` and do not pipe), the item-count guard, and the
skeleton diff. Waves 3, 4 and 7 each move two modules. If a gate reds, split that wave into
one commit per module.

## Explicitly out of scope

- **`lazy_static` → `std::sync::LazyLock`.** This is a worthwhile follow-on (it removes the
  macro-recursion ceiling for good), but it is a semantic change and must not ride inside a
  motion wave. File it separately if wanted after Wave 8.
- Renaming any metric, label, or label value. That is a wire change and needs its own
  dashboard/alert migration.
- The prometheus 0.14 legacy-getter deprecation
  ([deprecation-prometheus-014-proto-ext-legacy-getters](epr:deprecation-prometheus-014-proto-ext-legacy-getters)).
  Do it before or after, never inside a wave.
- New instruments. [no-latency-metric-for-change-doorbell](epr:no-latency-metric-for-change-doorbell)
  and [transport-route-metrics-pretouch-zero](epr:transport-route-metrics-pretouch-zero) land
  into whatever shape exists when they ship. After Wave 8 they each touch exactly one module.

## Readiness

- **Blocked** until the three in-flight commits that edit the file (`19c78d4af`, `fef7f7702`,
  `ef752ee26`; see the 2026-09-23 re-measure) are on `dev`. Check with
  `git log --oneline dev..<branch> -- elohim/elohim-storage/src/metrics.rs` across the live
  sprint branches. It is unblocked when that shows nothing, or only commits you have decided to
  carry forward over the motion. A motion wave on a file that is still being edited guarantees
  merge conflicts.
- **Do not wait for a quiet window.** With the edit spread across about twenty worktrees, one
  will not open. As soon as the three commits land, run Wave 0 and Wave 1 together, then
  announce the module layout. Pull the Wave 9 "where a new instrument goes" module-doc
  paragraph forward into Wave 1. From then on, each branch that adds an instrument writes it
  into its concern module, so later merges conflict in one small file rather than the root,
  and the root stops growing. Waves 2–8 follow, one per integration cycle.
- The finding reaches the `dev` ledger as soon as `dev` crosses 7,000, which happens when the
  first two commits land. Treat that as the start signal, not as a new finding. The same
  fingerprint `fe9edbbb11cd` maps to this entry.
- Effort: M. That is nine small commits, and none of them needs design judgment except Wave 8.
- Owner: a rust-architect shift. Waves 1–7 can go to a Sonnet-tier implementer under the
  invariants above. Wave 8 and the skeleton harness need an Opus-tier reviewer who reads the
  actual diff, not the subagent's report.
