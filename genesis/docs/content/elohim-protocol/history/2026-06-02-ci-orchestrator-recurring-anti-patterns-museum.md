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
> These thirteen failure modes recurred across **≥3 distinct agentic shifts each** (the starred rows are
> sub-threshold but earned their place — see the footnote) — recurrence is the signal
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
| 11 | **`net.sf.json.JSONNull` is Groovy-truthy — a `!meta.field` filter leaks** — a registry/map round-tripped through `writeJSON`→`readJSON` (e.g. `env.PIPELINE_REGISTRY_JSON`) turns an absent/null field into `JSONNull.getInstance()`, a non-null OBJECT. So a raw `!meta.jenkinsPath` exclusion filter does NOT exclude — a graph-only pipeline (`elohim-doorway-app`, `elohim-compute`; no `jenkinsPath` by design) survives the Determine-Build-Plan filter, gets dispatched, throws `No item named …/dev found` → soft-skip → permanent UNSTABLE. Mirror the JS contract (`typeof === 'string' && length > 0`); normalize JSONNull/`'null'`/`''` to real null at the registry boundary. | 1* | `backlog/ci-orchestrator-graph-only-pipeline-dispatch-leak.md`; JS oracle `genesis/orchestrator/pipeline-registry.mjs` `dispatchablePipelines` + `pipeline-registry.test.mjs` |

| 12 | **Prefix-matched "first-reachable-wins" is not a routing strategy** — when EVERY member of a pool satisfies the match predicate, first-that-matches silently degenerates to **index 0 always wins**. The genesis seeders walk `CONDUCTOR_URLS` looking for an installed app whose id starts with `elohim` — but every alpha conductor installs one, so the walk always stopped at adam. It reads as a fleet-wide outage whenever index 0 is the unhealthy member, and as *correct behavior* whenever index 0 happens to be healthy — while writing every human's data onto index 0's source chain with index 0's provenance. Fix: name-affinity (`elohim-<name>-<env>`) **plus** an explicit `skipped` state for "no pod for this member" — never a broader walk. **Diagnostic tell: every failing row names the same target.** | 2* | `backlog/ci-genesis-household-founder-binding.md` (#1119, identities); `backlog/ci-genesis-agent-bindings-conductor-fanin.md` (#1380–#1386, bindings — the sibling that never got the backport) |

| 13 | **A `DEFERRED:` fallback arm in the pre-push hook is DEAD CODE — the local gate is whatever the justfile says, and nothing else** — `run_gate` branches `elif command -v just && [ -f justfile ]` → `just gate`, *else* the big `case` block. `just` IS installed in the dev image, so for any project with a justfile the `case` arm never executes. Its `DEFERRED:` comments ("fallback includes X that the justfile gate omits") therefore describe **coverage nobody gets** — and read as reassurance while the gap is total. Compounding tell: `vitest run` does not type-check (vite strips types), so a `test`-only gate is blind to every strict-TS error in a spec file while CI's `tsc --noEmit` sees the whole tree. Diagnostic question when a red "should have been caught locally": *is the step in the justfile `gate` recipe, or only in the fallback arm?* Fix: put the step in the justfile; never park it in the fallback. | 2* | `backlog/ci-genesis-seeder-spec-typecheck-gate-gap.md` (#1400–#1401, TS2352/TS2493); `backlog/ci-genesis-projectionspec-ts2739.md` (#1101, TS2739 — same stage, same blind spot, two months earlier) |

| 14 | **The measure OVER-reads: a findings-taxonomy token with no error context fingerprints the benign output of a SUCCEEDING step** — trap #1's opposite polarity, on the sentinel arm instead of the pipeline. A bare tool/verb token (`nerdctl`, `rollout`) matches lines that every *healthy* build emits: `set -x` command echoes of a cleanup that worked, and the progress narration `kubectl rollout status` prints two lines above `deployment … successfully rolled out`. Because a stage-level `unstable()` yields no JUnit cases, `ci-harvest` falls through to the console-tail scan, where `MAX_CONSOLE_FINDINGS_PER_BUILD = 4` is spent on the noise — so the false findings don't merely add up, they **crowd out the real cause in the same tail** and each new permutation costs a background triage dispatch. **Diagnostic tell: the captured line is a *progress* or *command* line, and the line 1–3 below it says the step succeeded.** Fix in three layers: the taxonomy token must carry error context (`rollout.*(?:failed\|timed out\|exceeded)`, not `rollout`); a class-level guard skips command echoes (`_CMD_ECHO`) and progress/success chatter (`_BENIGN_PROGRESS`); a test asserts both non-capture of the benign lines AND still-capture of the real failure shapes. Safe because a genuine stall emits a SEPARATE non-progress error line. | 2* | `backlog/ci-harvest-nerdctl-cleanup-echo-overcapture.md` (#1137, INFRASTRUCTURE/`nerdctl`); `backlog/ci-harvest-rollout-progress-overcapture.md` (#1195–#1293, DEPLOYMENT/`rollout` — second category, the surface the first fix could not reach) |

\* #6 recurred in 2 shifts but is a full-cycle-cost no-op silencer worth the museum row.
#12 is likewise 2 occurrences, but of an *identical mechanism against the identical config list*, where
the second instance ran three weeks intermittently and its GREEN builds were the mis-provenanced ones —
a silent-corruption failure mode worth the row before a third recurrence. #11 is a first-occurrence (fp `97d7fb9c085c`, #1185–#1197) earning its row as a *new structural class* — JSONNull-survives-truthiness after a writeJSON/readJSON round-trip — not yet a ≥3-shift recurrence; recorded so the next planner does not re-derive it. (Same `net.sf.json` library as the JSONArray note at `Jenkinsfile:1654`.)
#13 is 2 occurrences (#1101 and #1400–#1401) two months and two authors apart against the *same* CI
stage through the *same* dead-gate mechanism, and it is a **meta-trap**: it is the reason other traps in
this table keep reaching CI at all, so it earns the row before a third recurrence. Its blast radius is
wider than the one gate that was fixed — at the time of writing, `elohim-app` (fallback claims full-tree
`eslint`; justfile `gate` runs only `lint-routes lint-a11y`) and `orchestrator` (fallback claims
`graph-walker` + `orchestrator-integration` node tests; `test-jenkinsfile-lints` is scoped to
`jenkinsfile-cps-scope.test.mjs` alone) carry the identical dead claim. Audit every `DEFERRED:` note in
`.husky/pre-push.bash` against the project's justfile `gate` recipe before trusting it.
#14 is 2 occurrences a month apart, and it earns the row before a third on two grounds. First, the
second instance proved the class is **wider than its first fix**: the `_CMD_ECHO` guard that closed the
`nerdctl` echoes structurally could not touch rollout progress lines (step output carries no `+ `
prefix), so "we fixed that" was false comfort. Second, like #13 it is a **meta-trap** — it degrades the
apparatus an agent uses to see CI at all, and its harm is not additive but *displacing*: a four-finding
per-build cap spent on benign chatter hides the real cause in the same log tail, which is how #1291's
actual RBAC-drift `unstable()` went unfiled while four healthy rollouts got fingerprinted. When a
findings entry looks strange, read the three lines BELOW the captured line before believing it.

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
