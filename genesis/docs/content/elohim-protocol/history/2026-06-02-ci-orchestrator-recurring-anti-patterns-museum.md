---
title: "History/ADR: CI / orchestrator recurring anti-patterns — the museum face"
id: ci-orchestrator-recurring-anti-patterns-museum
type: history-gotcha
status: Accepted
tier: history
created: 2026-06-02
topic: [ci, orchestrator, jenkins, sccache, husky, sweettest, cucumber, anti-patterns]
# This is the CURATED museum face of the frequency-ranked CI/orchestrator anti-patterns
# surfaced by repeated agentic shifts (≥3-shift recurrences). It is NOT a relocated journal:
# each pattern's mechanism lives in a live memory entry (linked); this record is the single
# place a future planner meets the whole frequency-ranked set. Shift narration bodies retire
# to git; the recurring patterns earn this museum record per the shifts-decomposition rule.
canonical:
  - ../../../../orchestrator/README.md   # genesis/orchestrator/README.md — the canonical orchestrator doc
memory_anchors:
  - feedback_orchestrator_abort_baseline_rollback
  - feedback_dockerfile_target_completeness
  - feedback_orchestrator_build_manifest_required
  - feedback_husky_bypass_for_ci_only_changes
  - feedback_sccache_cache_corruption_recovery
  - feedback_sccache_spawn_enoent_rca
  - feedback_cascade_halt_masks_failures
  - feedback_cascade_hidden_test_surface
  - project_pre_dispatch_hard_fail_post_dispatch_unstable
  - feedback_cargo_nextest_installed
---

# CI / orchestrator recurring anti-patterns — the museum face

> **Hot-context pointer (the one sentence to remember):**
> These ten failure modes recurred across **≥3 distinct agentic shifts each** — recurrence is the signal
> that an anti-pattern earned canonical placement, not just narration. The mechanism of each lives in a
> linked memory entry; this record is the **single frequency-ranked index** a future CI planner meets
> before touching the pipeline. Read it before you read a red build as a regression.

## Why this record exists

Agentic developer shifts (the ~14-day working-memory tier) repeatedly hit the same CI/orchestrator
gotchas. Per the shifts-decomposition rule, a recurring anti-pattern must land in the **history museum**
*and* in its canonical surface — not only in a scattered memory entry. A one-off is narration; a
≥3-shift recurrence is structure. This is the curated museum face of that structure. The shift narration
bodies themselves retire to git; the durable mechanism of each pattern lives in the linked memory entry.

## The frequency-ranked set (recurrence = distinct shifts the pattern appeared in)

| # | Anti-pattern (the trap) | Freq | Where the mechanism lives |
|---|---|---|---|
| 1 | **Orchestrator NOT_BUILT / superseded read as regression** — `abortPrevious` preempts an in-flight child; a FAILURE-count grep reads NOT_BUILT/ABORTED/UNSTABLE all as 0 (lossy). A superseded build is not a failure. | 8 | `feedback_orchestrator_abort_baseline_rollback`, `project_pre_dispatch_hard_fail_post_dispatch_unstable` |
| 2 | **Baseline-rollback over-build** — a FAILURE/ABORT invalidates the per-pipeline baseline → reverts to the global baseline → full cascade; `lastSuccessful()` pins an ancient green. | 6 | `feedback_orchestrator_abort_baseline_rollback` |
| 3 | **Dockerfile / build-manifest completeness** — a new Cargo target OR a new path-dep crate breaks the Docker build context but passes host pre-push; the manifest under-covers source inputs so the orchestrator under-dispatches. | 6 | `feedback_dockerfile_target_completeness`, `feedback_orchestrator_build_manifest_required` |
| 4 | **HUSKY=0 is NON-FUNCTIONAL** — `core.hooksPath=.husky` bypasses the wrapper that honors `HUSKY=0`; the real bypass is `git push --no-verify`. (Root `CLAUDE.md` corrected 2026-06-02.) | 4 | `feedback_husky_bypass_for_ci_only_changes` |
| 5 | **sccache / S3 cache poisons rustc output** — a `NoSuchKey`/null-byte interleaves into diagnostics → spurious "unclosed delimiter"; `RUSTC_WRAPPER=""` bypasses but the poisoned key persists (heal = `SCCACHE_RECACHE=1` / repave). | 4 | `feedback_sccache_cache_corruption_recovery`, `feedback_sccache_spawn_enoent_rca` |
| 6 | **`#[ignore]` is a CI no-op** — the DNA sweettest stage runs `cargo nextest run --run-ignored all`; quarantine-by-`#[ignore]` still runs (and still fails) in CI, costing a full ~75-min cycle. Delete the test or change the invocation — do not annotate. | 2* | `feedback_cargo_nextest_installed` (+ the sweettest stage invocation in the DNA Jenkinsfile) |
| 7 | **Cucumber / Gherkin parse aborts the whole E2E run** — an unescaped `/` → empty-alternation; a bare continuation line → AST reject; an empty cucumber-report → UNSTABLE with a blank body. Read the E2E log FIRST. | 4 | (a2o framework conventions; backlog: pre-push gherkin linter) |
| 8 | **CPS method-size limit** — the ~64KB/11000-byte hard cap; inline handlers / pre-flight blocks blow it; extract a helper above `pipeline {}`. (A CPS-scope-loss-across-stages variant — env-bridge, not method-size — is a separate trap.) | 4 | root `CLAUDE.md` "Jenkinsfile Size Limit" gotcha; `jenkinsfile-cps-scope.test.mjs` |
| 9 | **Webhook double-fire** — one dev push → 2 builds (the first superseded); an explicit `triggers{githubPush()}` AND the Multibranch implicit trigger both fire. | 4 | (orchestrator README triggers section; backlog: remove the explicit trigger) |
| 10 | **Cascade-halt / cascade-hidden test surface** — driving a long-red pipeline green unmasks buried failures one layer at a time; track the pass *ratio*, not the raw count, and budget extra iterations. | 5 | `feedback_cascade_halt_masks_failures`, `feedback_cascade_hidden_test_surface` |

\* #6 recurred in 2 shifts but is a full-cycle-cost no-op silencer worth the museum row.

## The load-bearing reading (so you feel the pull and resist it)

The single deepest trap is **#1/#2**: an agent reads a red/NOT_BUILT/ABORTED orchestrator result as
"something I broke" and either re-dispatches the world or rolls the baseline back to an ancient green —
amplifying the cascade. NOT_BUILT and superseded are *not* failures; a FAILURE-count grep that flattens
them is lossy. Confirm against the last **green** commit, not the last *landed* one, and read the actual
child-build result before treating it as a regression. (This is the same baseline-drift mechanism the
`deploy-is-not-a-graph-node` record diagnoses from the incident side.)

The second cluster (**#3/#5/#6**) is the same shape under three disguises: a check that passes on the
host but fails in CI because the CI environment differs (Docker context, sccache wrapper, `--run-ignored
all`). Host-green ≠ CI-green; the gap is the environment, not your code.

## Watch-out for future CI planners

- **Do NOT edit any Jenkinsfile to "fix" these from this record.** The root `Jenkinsfile` is near the
  64KB CPS limit; orchestrator/CI watch-outs route to DOCS — `genesis/orchestrator/README.md`, the root
  `CLAUDE.md` CI/CD section, and the a2o/DNA conventions — never inline pipeline logic.
- The open *code-domain* hardening items these patterns imply (orchestrator measure-tightening, baseline
  state-machine, manifest⊆strategy drift test, Dockerfile-completeness lint, pre-push gherkin linter,
  sccache hardening, trigger-dedup, CPS-scope static lint) are promoted to `genesis/data/timeline/backlog/`
  — they are resolved into backlog items, not buried in shift narration.
- The *operator-domain* items (jenkins-deployer RBAC drift, Harbor registry SPOF, cross-ns NetworkPolicy,
  checkout reliability, cluster pressure) are operator-owned and likewise routed to backlog.
- **Fail-regime boundary (when you *do* touch the orchestrator Jenkinsfile for a real fix):** stages
  *before* dispatch (e.g. `Post Predicted Build Graph`) MAY hard-fail — a broken setup should stop
  dispatch loudly. Stages *after* dispatch are observational (`Post Actual Build Graph`, `Verify
  Deployment`, `Post-flight Health Check`, `Reconcile Build Graph`) — the world is already what it is, so
  a parse/archive/HTTP hiccup there must `catchError(buildResult: 'UNSTABLE', stageResult: 'UNSTABLE')`,
  never FAILURE. A FAILURE'd orchestrator blanks the downstream truth those stages exist to surface and
  forces the agentic loop to retry blind; UNSTABLE preserves the truth and still flags the hiccup.

## Bidirectional links

- **This record → canonical:** the [orchestrator README](../../../../orchestrator/README.md) (the canonical orchestrator doc — dispatch, NOT_BUILT, baseline, trigger semantics).
- **Sibling record:** [deploy-is-not-a-graph-node](2026-06-02-deploy-is-not-a-graph-node.md) (the baseline-drift / deploy-dispatch incident that anchors patterns #1/#2).
- **Mechanism (live memory entries):** linked per-pattern in the table above and in `memory_anchors`.
