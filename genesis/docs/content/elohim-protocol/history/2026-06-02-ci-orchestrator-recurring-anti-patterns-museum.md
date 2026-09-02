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
> These seventeen failure modes recurred across **≥3 distinct agentic shifts each** (the starred rows are
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

| 15 | **A Jenkins CONTROLLER RESTART orphans the whole dispatched wave — and trap #1's advice is exactly backwards for it** — the controller goes down mid-wave; on resume every in-flight agent pod is gone (`Waiting for reconnection of <pod> before proceeding with build`, `Could not connect to … to send interrupt signal to process`). Three symptoms, one cause, and they look unrelated: the downstream child ends **ABORTED**, a sibling ends **FAILURE**, and the orchestrator ends FAILURE with a **fabricated Groovy syntax error** — `MultipleCompilationErrorsException` naming a line that is valid and that the job's very next build compiles clean, because `FlowExecutionList.resume → loadProgramAsync → parseScript` reparsed a CPS script the hard restart never flushed (tell: the echoed source line **stops mid-token**). Compounding: a declarative `timeout` counts **through the outage**, so a 2h budget is spent entirely on an idle build and fires the instant it resumes — work destroyed by a timer that measured downtime. **Discriminator vs #1: `Resuming build … after Jenkins restart` in the log = restart-orphaned → RETRIGGER (the work was never done); an `abortPrevious` preemption by a newer build number = superseded → IGNORE.** Same ABORTED symptom, opposite remedy. Never edit a Jenkinsfile in response — check whether the job's NEXT build compiles the same file first. | 1* | `backlog/ci-jenkins-controller-restart-orphans-wave.md` (#1669/#1343/#1664, fp `2ec906730fe7`); sibling agent-pod-channel class `backlog/ci-jenkins-k8s-pod-exec-websocket-transient.md` |

| 16 | **Measurement-by-restart — a stage bounces the very surface it is about to measure, and the scaffold that justified the bounce was self-healed away months ago** — `seedProjectionsStage()` pod-deletes doorway-alpha on EVERY genesis run to force an EprRouter refresh, then the E2E stage measures that pod ~6 minutes later. The delete resets the doorway's p2p snapshot cache and its upstream circuit breakers, so E2E reads **recovery, not truth**: `p2p.caughtUp` absent (no snapshot yet — a *different* JSON shape from `caughtUp: false`, which means a snapshot exists and reports behind) and `status=degraded` (the `serving.shedding \|\| serving.degrading` override on the declared primary). The scaffold's stated premise — "the router only refreshes at boot OR via SSE" — was already false when written: `doorway-service` `main.rs` has run a periodic EPR-router self-heal refresh every `DOORWAY_EPR_REFRESH_SECS` (default 30) since `379668123` (2026-05-30), executing the byte-for-byte boot sequence and saying so in its own comment ("no kubectl restart needed"); the Jenkinsfile comment dates from 2026-06-10. **Diagnostic tell: the failing assertion reads a freshness/liveness field of a surface THIS pipeline restarted earlier in the same run.** Before writing or keeping any restart-to-refresh step, grep the runtime for a periodic refresh loop; if one exists, the step's job is to *wait out two ticks and verify*, not to delete a pod. Same family as the root `CLAUDE.md` "measurement-by-deploy" note for a bare `[build:edge]` fired just to measure — that one is operator-facing, this one is in-tree and fires every run. | 1* | `backlog/ci-genesis-doorway-503-seed-phase-wedge.md` (#1512–#1514, fps `a672ee4586c6` / `193b7597a4cb` / `4b6fe47bfdb3`); `genesis/scripts/ci/restart-doorway-epr.sh`; root `CLAUDE.md` §Force dispatch |
| 17 | **A generated-twin freshness gate keyed to the GENERATOR's directory never fires — the edit that breaks it lives in the SOURCE's directory, which selects a different gate project (or none)** — the guard is written, correct, and structurally unreachable locally, so CI is its first observer. `cid-artifact-integrity.spec.ts` lives in `genesis/seeder`, but the `.md` whose bytes it hashes lives in `genesis/docs/content/elohim-protocol/` — a path claimed by `app/elohim-library/build-manifest.json` (Storybook consumes protocol docs as story sources), so `gate-runner --changed-file-list --names` on `manifesto.md` prints `elohim-storybook` and the seeder spec never runs. `manifesto.json`'s `blobHash`/`blobCid` dangled through seven `manifesto.md` commits until genesis #1522 stopped at Validate Constants — before Seed Database, so nothing seeded and the live head silently stayed old. The tell that this is a *class*, not an incident: `.husky/pre-push.bash` already carries four hand-wired source-side legs (humans/presences, device archetypes, deployments, account packages) written one incident at a time; CID artifacts were the fifth twin and the one nobody wired. **Rule: a freshness check belongs to the SOURCE glob, never the generator's.** Diagnostic: pipe the source path through `gate-runner.mjs --changed-file-list --names` and see whether the owning project is even named. Second edge, worth its own reflex: **`pnpm --filter <unknown-name> …` prints `No projects matched the filters` and EXITS 0** — every remediation hint in the repo naming `genesis-seeder` (the package is `holochain-seeder`) was a copy-pasteable no-op that reads as success. | 5* | `backlog/ci-cid-artifact-twin-source-side-freshness-gap.md` (#1522, fps `116c98ba145a`/`844752df1596`); the four pre-legs in `.husky/pre-push.bash`; adjacency: #13 (gate-coverage meta-trap) |

| 18 | **A per-item "partial failure is just a test failure" posture, correct for an INDEPENDENT phase, is inherited by a SHARED-ARTIFACT phase — where the first failure is a leading indicator for every item behind it, so continuing walks the whole fleet** — and the safety pause is conditioned on success, so the loop accelerates exactly when it should stop. `deployHumansInParallel` (`elohim/holochain/Jenkinsfile`) runs two phases. The parallel STORAGE phase's continue-on-failure/UNSTABLE-not-FAILURE rationale is sound and documented in the function's own doc comment (a peer's resource floor is a per-peer test failure — the 2026-05-22 james OOM). The sequential rung-3 CONDUCTOR phase inherited it, but a conductor only rolls when a SHARED artifact moved (hApp digest / conductor pin), so a peer whose conductor will not come Ready predicts the next one. On 2026-09-02 the phase walked `ordered` — non-genesis first, matthew last — at one `--timeout=600s` per peer, rolling six more healthy conductors into a state whose first instance had already crash-looped; each freshly rolled node reinstalled a drifted DNA and tore its source chain. **Diagnostic tells, in order of cheapness:** (a) the failing items appear in the loop's own declared order at intervals equal to the loop's own timeout — the cadence IS the pipeline, not the substrate; (b) `getBuild tree=artifacts[relativePath]` shows evidence bundles for MULTIPLE items from ONE build (#1413 archived four conductor bundles) — that list is a picture of a loop that never halted; (c) the soak/backoff sits AFTER the throw point, so a failing item advances faster than a healthy one. **Rule: continue-on-failure is a property of the phase's independence, never of the file it is written in — a shared-artifact roll halts on first failure and leaves the rest on their last-known-good.** The escape hatch is an env flag an operator sets deliberately, never the default. | 1* | `backlog/ci-edge-conductor-roll-no-halt-walks-the-fleet.md` (#1413/#1414, fps `ca8705d89578`/`e63e3dcd0771`/`d89606ca5cde`); `backlog/alpha-conductor-crash-loop-after-wave4-roll-and-moved-dna-hashes.md` (the substrate half); adjacency: #16 (a pipeline that inflicts what it reports) |

#17's frequency counts the *shape*, not the symptom: four prior generated twins (humans, presences,
device archetypes, deployments/account-packages) each got a bespoke source-side pre-push leg written
reactively, and the fifth (CID artifacts) reached CI because nobody generalized. It earns the row as a
**structural class with a mechanical diagnostic** — pipe the source path through the gate-runner and
read whether the owning project is named — rather than as a fifth recurrence of #13. #13 asks *is the
step in the justfile gate or only the fallback arm?*; #17 asks *does the step's trigger cover the file
the AUTHOR edits?* A gate can be in the justfile, correct, fast, and still structurally unreachable.

\* #6 recurred in 2 shifts but is a full-cycle-cost no-op silencer worth the museum row.
#12 is likewise 2 occurrences, but of an *identical mechanism against the identical config list*, where
the second instance ran three weeks intermittently and its GREEN builds were the mis-provenanced ones —
a silent-corruption failure mode worth the row before a third recurrence. #11 is a first-occurrence (fp `97d7fb9c085c`, #1185–#1197) earning its row as a *new structural class* — JSONNull-survives-truthiness after a writeJSON/readJSON round-trip — not yet a ≥3-shift recurrence; recorded so the next planner does not re-derive it. (Same `net.sf.json` library as the JSONArray note at `Jenkinsfile:1654`.)
#15 is a first-occurrence (fp `2ec906730fe7`, 2026-08-13) earning its row on the same ground #11 did — a
*new structural class*, not a recurrence of #1. It is recorded before a second occurrence because it is
the one entry in this table whose symptom is already covered by another row while its **remedy is the
exact opposite**: #1 teaches "ABORTED is not a failure, do nothing", which is right for a superseded
build and wrong for a restart-orphaned one whose work was destroyed and never redone. An agent holding
only #1 will correctly refuse to panic and then incorrectly refuse to retrigger. Its second hazard is
worse than a wasted cycle: a fabricated Groovy compile error invites a "fix" to a Jenkinsfile that was
never broken and is near the CPS size limit (#8).
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
#16 is a first-occurrence earning its row on the same ground #11 and #15 did — a *new structural
class*, recorded before a second instance because the class is self-concealing in a specific way: the
pipeline that inflicts the damage is also the one that reports it, so every red it produces looks like
a substrate red and gets triaged as one. Three builds (#1512, #1513, #1514) each had their
`caughtUp` / `degraded` fingerprints read as alpha-cluster degradation before anyone checked what the
pipeline had done to the doorway minutes earlier. Its second hazard is the more general one and is why
the row is worded around *scaffolds*, not doorways: a workaround written against a real runtime gap
does not expire when the gap is closed. The runtime here fixed itself in May and said so in a code
comment; the pipeline kept paying for a cure it no longer needed, and the payment was denominated in
exactly the measure the pipeline exists to produce. When a pipeline step's comment asserts a runtime
limitation, date the assertion against the runtime — a stale premise in a comment is
indistinguishable from a live one at read time.

#18 is a first-occurrence earning its row on the ground #15 established: its remedy is the **exact
opposite** of a rationale that is already written down, in prose, in the very function it applies to —
and that rationale is *correct where it was written*. An agent who reads
`deployHumansInParallel`'s doc comment ("a partial cluster-side failure … marks the parent stage
UNSTABLE instead of FAILURE … a single peer's resource floor being too tight is a per-peer test
failure, not a pipeline-broken-shape problem") will conclude the continue-on-failure posture is
deliberate and defended, because for the parallel storage phase it is. Nothing in the file marks
where that reasoning stops holding. The blast radius of getting it wrong is not a wasted cycle: the
2026-09-02 walk destroyed the source chains of seven peers, irrecoverably, one 600s timeout at a
time, and every subsequent edge build re-entered the same loop. The row is worded around
*shared-artifact phases*, not conductors, because the discriminator generalizes — before trusting a
loop's continue-on-failure, ask what makes the items independent, and whether the thing that
triggered the loop is per-item or shared. If it is shared, the first failure is a forecast.

## The load-bearing reading (so you feel the pull and resist it)

The single deepest trap is **#1/#2**: an agent reads a red/NOT_BUILT/ABORTED orchestrator result as
"something I broke" and either re-dispatches the world or rolls the baseline back to an ancient green —
amplifying the cascade. NOT_BUILT and superseded are *not* failures; a FAILURE-count grep that flattens
them is lossy. Confirm against the last **green** commit, not the last *landed* one, and read the actual
child-build result before treating it as a regression. (This is the same baseline-drift mechanism the
`deploy-is-not-a-graph-node` record diagnoses from the incident side.)

**But do not over-apply it — read #15 before concluding "ABORTED, therefore nothing to do."** That
inference holds only when the abort was a *preemption* (a newer build superseded this one). When the
abort came from a **controller restart**, the identical symptom means the opposite: the work was
destroyed and never redone, so a retrigger is required, not withheld. The log settles it in one line —
`Resuming build … after Jenkins restart` (retrigger) versus a newer build number having preempted this
one (ignore).

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
- **Pattern #16 → live concern:** `genesis/data/timeline/backlog/ci-genesis-doorway-503-seed-phase-wedge.md` (the genesis measurement-by-restart entry, where the cure and the remaining substrate leg are tracked).
- **Pattern #17 → live concern:** `genesis/data/timeline/backlog/ci-cid-artifact-twin-source-side-freshness-gap.md` (the CID-twin drift and the source-side pre-push rail that closes it).
