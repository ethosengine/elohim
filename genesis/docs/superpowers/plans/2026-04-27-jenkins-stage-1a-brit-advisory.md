# Jenkins Stage 1a — Brit Advisory Wire-Up Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire brit advisory checks into the developer pre-push hook and the Jenkins orchestrator as Stage 1a of the Jenkins-as-brit-attestation-producer migration. Delivers immediate value to developers (5s local plan preview before push) and seeds the orchestrator with advisory comparison data, while deferring the ci-builder image rebuild (Stage 1b) and per-pipeline post.success attestation writes (Stage 1c) to follow-up plans.

**Architecture:** Three additive changes. (1) Pre-push hook builds brit-build-ref + brit-cli locally on first invocation if not already built, then runs `brit verify` and `brit plan --since origin/dev` as advisory (warn-only). (2) Orchestrator Jenkinsfile gains an advisory stage that runs `brit plan` if the brit binary is installed on the agent (gracefully no-ops with a log warning if not — until Stage 1b ships brit on the ci-builder image). (3) Documentation captures the migration roadmap and Stage 1b/1c prerequisites.

**Tech Stack:** Rust 2021 (brit-build-ref / brit-cli compile via cargo), POSIX shell (.husky/pre-push), Jenkins declarative pipeline Groovy, npm-groovy-lint (existing pre-push gate validates Jenkinsfile changes).

**Spec:** `genesis/docs/superpowers/specs/2026-04-27-jenkins-as-brit-attestation-producer-design.md` — when this plan and the spec disagree, the spec wins.

**Companion specs (referenced, not modified):**
- `elohim/brit/docs/specs/2026-04-27-build-contract-before-push-design.md`
- `elohim/rakia/docs/specs/2026-04-27-rakia-as-brit-attestation-executor-design.md`

**Out of scope (separate plans):**
- Stage 1b: ci-builder image rebuild to install brit-build-ref + brit-cli (operator action, separate `ee-jenkins-ci-builder` repo)
- Stage 1c: per-pipeline post.success attestation writes (e.g., elohim-edge as test bed)
- Stage 2: consume brit plan for dispatch decisions
- Stage 3: retire legacy `build-graph.groovy`, `pipeline-baselines.json`, `analyzePipelineRequirements`

---

## File Structure

| File | Purpose | Action |
|---|---|---|
| `.husky/pre-push` | Existing 590-line hook with project-specific change-detection gates | Modify: add `brit_advisory()` function + invoke after existing gates |
| `genesis/orchestrator/Jenkinsfile` | Orchestrator pipeline (1500+ lines) | Modify: add `Compute brit Plan (advisory)` stage after existing planning |
| `genesis/orchestrator/scripts/brit-helper.sh` | NEW — wrapper around brit calls with safe-fallback semantics | Create |
| `genesis/docs/superpowers/plans/2026-04-27-jenkins-stage-1a-brit-advisory.md` | This plan | (already created — meta) |
| `genesis/docs/integrations/brit-migration-roadmap.md` | NEW — captures Stage 1a/1b/1c/2/3 sequencing for operator + developer reference | Create |

---

## Task 1: Verify brit-build-ref + brit-cli compile locally

**Files:** none modified — verification only.

- [ ] **Step 1: Confirm brit submodule is at expected commit**

Run: `cd /projects/elohim && git submodule status elohim/brit`
Expected: `7a51e1ff2ea1249402905bd479230d151defecc0 elohim/brit (heads/main)`

If the SHA differs (other landings on brit/main), fast-forward + bump parent pointer per the brit harvest spec; not required for Stage 1a but worth noting.

- [ ] **Step 2: Build both binaries from the brit submodule**

Run: `cd /projects/elohim/elohim/brit && cargo build --release -p brit-build-ref -p brit-cli 2>&1 | tail -5`
Expected: `Finished \`release\` profile [optimized] target(s) in <duration>`

If compile fails, file an issue against brit and stop this plan — Stage 1a depends on these binaries being buildable.

- [ ] **Step 3: Smoke-test the binaries**

Run: `/projects/elohim/elohim/brit/target/release/brit-build-ref --help 2>&1 | head -5`
Expected output begins with: `Manage build/deploy/validate/reach attestation refs`

Run: `/projects/elohim/elohim/brit/target/release/brit --help 2>&1 | head -5` (note: brit-cli's binary may be named `brit` or similar — adjust based on actual `Cargo.toml` `[[bin]]` name)
Expected: clap-formatted usage output with subcommands `plan`, `affected`, `fingerprint`, etc.

If the binary name differs from `brit`, note it for use in subsequent tasks; substitute the actual name everywhere this plan says `brit`.

---

## Task 2: Create brit-helper.sh with safe-fallback semantics

**Files:**
- Create: `genesis/orchestrator/scripts/brit-helper.sh`

The helper wraps brit calls so missing-binary scenarios (Stage 1a — before ci-builder image carries brit) degrade to advisory log lines instead of failing the build.

- [ ] **Step 1: Create the helper script**

Path: `/projects/elohim/genesis/orchestrator/scripts/brit-helper.sh`

```bash
#!/usr/bin/env sh
# brit-helper.sh — safe wrapper for brit CLI invocations during Stage 1a/1b migration.
#
# Stage 1a: brit not yet installed on ci-builder; helper logs WARN and exits 0.
# Stage 1b: brit installed; helper invokes brit and forwards its exit code.
# Stage 2:  helper retired in favor of direct brit calls (when failure should be load-bearing).
#
# Usage:
#   brit-helper.sh verify
#   brit-helper.sh plan --since refs/notes/brit/build-baselines/__global__
#   brit-helper.sh build-ref build put --step <name> --inputs-hash <hash> ...

set -e

# Locate brit binaries: prefer system PATH, fall back to local submodule build.
BRIT_BIN=""
if command -v brit >/dev/null 2>&1; then
    BRIT_BIN=brit
elif [ -x "${REPO_ROOT:-/projects/elohim}/elohim/brit/target/release/brit" ]; then
    BRIT_BIN="${REPO_ROOT:-/projects/elohim}/elohim/brit/target/release/brit"
fi

BRIT_BUILD_REF_BIN=""
if command -v brit-build-ref >/dev/null 2>&1; then
    BRIT_BUILD_REF_BIN=brit-build-ref
elif [ -x "${REPO_ROOT:-/projects/elohim}/elohim/brit/target/release/brit-build-ref" ]; then
    BRIT_BUILD_REF_BIN="${REPO_ROOT:-/projects/elohim}/elohim/brit/target/release/brit-build-ref"
fi

# Subcommand routing.
case "${1:-}" in
    verify)
        shift
        if [ -z "$BRIT_BIN" ]; then
            echo "[brit-helper] WARN: brit not installed; verify advisory skipped (Stage 1a)" >&2
            exit 0
        fi
        # brit verify is itself a stub today (Phase 2B); call it anyway so when it lands we get real output.
        echo "[brit-helper] running: $BRIT_BIN verify $*"
        "$BRIT_BIN" verify "$@" || {
            rc=$?
            echo "[brit-helper] WARN: brit verify exited $rc — advisory only, not failing the build" >&2
            exit 0
        }
        ;;
    plan)
        shift
        if [ -z "$BRIT_BIN" ]; then
            echo "[brit-helper] WARN: brit not installed; plan advisory skipped (Stage 1a)" >&2
            exit 0
        fi
        echo "[brit-helper] running: $BRIT_BIN plan $*"
        "$BRIT_BIN" plan "$@" || {
            rc=$?
            echo "[brit-helper] WARN: brit plan exited $rc — advisory only, not failing the build" >&2
            exit 0
        }
        ;;
    build-ref)
        shift
        if [ -z "$BRIT_BUILD_REF_BIN" ]; then
            echo "[brit-helper] WARN: brit-build-ref not installed; attestation skipped (Stage 1a)" >&2
            exit 0
        fi
        echo "[brit-helper] running: $BRIT_BUILD_REF_BIN $*"
        "$BRIT_BUILD_REF_BIN" "$@" || {
            rc=$?
            echo "[brit-helper] WARN: brit-build-ref exited $rc — advisory only, not failing the build" >&2
            exit 0
        }
        ;;
    *)
        echo "[brit-helper] usage: $0 {verify|plan|build-ref} [args...]" >&2
        exit 64
        ;;
esac
```

- [ ] **Step 2: Make it executable**

Run: `chmod +x /projects/elohim/genesis/orchestrator/scripts/brit-helper.sh`

- [ ] **Step 3: Smoke-test fallback path (no brit on PATH)**

Run: `PATH=/usr/bin:/bin REPO_ROOT=/projects/elohim /projects/elohim/genesis/orchestrator/scripts/brit-helper.sh verify`
Expected output: `[brit-helper] running: /projects/elohim/elohim/brit/target/release/brit verify` followed by either `WARN: brit verify exited <rc>` (if brit verify is stub) or actual verify output.

If `brit` doesn't exist at the local path either, you should see: `[brit-helper] WARN: brit not installed; verify advisory skipped (Stage 1a)` — exit 0.

- [ ] **Step 4: Smoke-test forwarding path (with brit on PATH)**

Run: `PATH="/projects/elohim/elohim/brit/target/release:$PATH" /projects/elohim/genesis/orchestrator/scripts/brit-helper.sh plan --files=README.md`
Expected: `[brit-helper] running: brit plan --files=README.md` followed by JSON output (a build plan) or warn-and-exit-0 if the planner errors.

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim
git add genesis/orchestrator/scripts/brit-helper.sh
git commit -m "$(cat <<'EOF'
feat(orchestrator): add brit-helper.sh wrapper for Stage 1a migration

Wraps brit / brit-build-ref invocations with safe-fallback semantics:
missing binary → log WARN → exit 0 (advisory mode for Stage 1a, before
ci-builder image carries brit). Becomes load-bearing in Stage 2 when
brit is required and the wrapper retires.

Companion to:
- genesis/docs/superpowers/specs/2026-04-27-jenkins-as-brit-attestation-producer-design.md
- genesis/docs/superpowers/plans/2026-04-27-jenkins-stage-1a-brit-advisory.md
EOF
)"
```

---

## Task 3: Add brit advisory to pre-push hook

**Files:**
- Modify: `.husky/pre-push` (currently 590 lines; add a function near the existing project-gating helpers and invoke it after the changed-project loop)

The advisory runs after existing per-project gates so the developer sees their own project's quality output first, then the brit predictions.

- [ ] **Step 1: Read current pre-push hook structure**

Run: `wc -l /projects/elohim/.husky/pre-push && grep -n '^[a-z_]*()' /projects/elohim/.husky/pre-push | head -20`
Note the function names and line numbers — the new function follows the existing style.

- [ ] **Step 2: Add brit_advisory function near existing helpers**

Locate the line just before the main script body (after all helper function definitions, before the `# ── Detect changed projects ──` block — typically around line 200-300; verify by grepping). Add the following function definition immediately before that section:

```bash
# ── Brit advisory (Stage 1a) ─────────────────────────────────────
# Runs brit verify + brit plan as advisory checks. Warn-only — never fails push.
# Stage 2: becomes load-bearing (failure blocks push). Stage 1a: gather signal.
brit_advisory() {
    HELPER="${REPO_ROOT}/genesis/orchestrator/scripts/brit-helper.sh"
    if [ ! -x "$HELPER" ]; then
        return 0  # helper not present (e.g., older branch); silently skip
    fi
    echo ""
    echo "── brit advisory (Stage 1a — warn-only) ─────────────────────"
    REPO_ROOT="$REPO_ROOT" "$HELPER" verify
    # plan against origin/dev (the merge target); if origin/dev is unreachable, skip.
    if git rev-parse --verify origin/dev >/dev/null 2>&1; then
        REPO_ROOT="$REPO_ROOT" "$HELPER" plan --since origin/dev
    else
        echo "[brit-helper] WARN: origin/dev not reachable; plan advisory skipped" >&2
    fi
    echo "── end brit advisory ─────────────────────────────────────────"
    return 0  # never fails the hook
}
```

- [ ] **Step 3: Invoke brit_advisory at the end of the hook**

Find the final `exit 0` (or last action of the hook). Add immediately before it:

```bash
# Stage 1a: advisory brit checks (never fail the push).
brit_advisory
```

- [ ] **Step 4: Verify pre-push hook syntactically valid**

Run: `sh -n /projects/elohim/.husky/pre-push && echo "syntax OK"`
Expected: `syntax OK`

- [ ] **Step 5: Manually run the hook to smoke-test (no actual push)**

Run from a state with at least one staged file or recent commit:

```bash
cd /projects/elohim
echo "test ref-update line" | REPO_ROOT=/projects/elohim sh .husky/pre-push 2>&1 | tail -30
```

Expected: existing project-gate output, then a `── brit advisory (Stage 1a — warn-only) ──` block, then `── end brit advisory ──`. Exit code: depends on whether other gates pass; brit_advisory itself should never push toward a failure.

- [ ] **Step 6: Commit**

```bash
cd /projects/elohim
git add .husky/pre-push
git commit -m "$(cat <<'EOF'
feat(husky): add brit advisory (Stage 1a) — warn-only verify + plan preview

Pre-push hook now invokes brit-helper.sh verify + plan --since origin/dev
after existing project-specific quality gates. Warn-only; never fails the
push. Developers get a 5-second local preview of what the orchestrator
would dispatch before pushing — closing the LLM-loop / dev-laptop loop
asymmetry that the brit harvest spec calls out.

Stage 2 makes brit verify load-bearing (failure blocks push). Stage 1a
seeds the signal.

Companion to:
- genesis/docs/superpowers/specs/2026-04-27-jenkins-as-brit-attestation-producer-design.md
- elohim/brit/docs/specs/2026-04-27-build-contract-before-push-design.md
EOF
)"
```

---

## Task 4: Add advisory brit plan stage to orchestrator Jenkinsfile

**Files:**
- Modify: `genesis/orchestrator/Jenkinsfile` — add a new stage after the existing `Plan & Decide` stage and before `Execute Builds`.

The advisory stage runs `brit plan` if available and prints divergence vs the legacy graph output. Does NOT alter dispatch (`env.PIPELINES_TO_RUN` is still set by the existing legacy + build-graph code path).

- [ ] **Step 1: Locate insertion point**

Run: `grep -n "stage('Execute Builds')" /projects/elohim/genesis/orchestrator/Jenkinsfile`
Note the line number. The new stage goes immediately above it.

- [ ] **Step 2: Add the advisory stage**

Insert the following BEFORE the `stage('Execute Builds')` block (preserving the existing surrounding indentation — stages are at 8-space indent inside `stages { }`):

```groovy
        stage('Brit Plan (advisory)') {
            when {
                expression {
                    // Only run when we have a planning result to compare against.
                    params.MODE != 'status' && fileExists('genesis/orchestrator/scripts/brit-helper.sh')
                }
            }
            steps {
                container('builder') {
                    script {
                        // brit-helper.sh wraps brit invocations with safe-fallback (logs WARN, exits 0
                        // if brit is not yet on the ci-builder image — Stage 1a behavior). Once Stage 1b
                        // lands brit on ci-builder, this stage produces a real advisory plan.
                        sh '''
                            export REPO_ROOT=$WORKSPACE
                            chmod +x genesis/orchestrator/scripts/brit-helper.sh || true
                            echo "── brit advisory (Stage 1a — warn-only) ──"
                            ./genesis/orchestrator/scripts/brit-helper.sh verify || true
                            ./genesis/orchestrator/scripts/brit-helper.sh plan --since origin/dev || true
                            echo "── end brit advisory ──"
                        '''
                        // Compare to the legacy/graph dispatch decision for log-side observation.
                        echo "ADVISORY: legacy + build-graph dispatched: ${env.PIPELINES_TO_RUN ?: '(none)'}"
                    }
                }
            }
        }

```

(Note the trailing blank line — keeps `Execute Builds` separated visually.)

- [ ] **Step 3: Lint the Jenkinsfile**

Run: `cd /projects/elohim && npm-groovy-lint --noserver --failon error genesis/orchestrator/Jenkinsfile 2>&1 | tail -10`
Expected: `0 errors` and ideally `0 warnings`. If warnings appear, address them before commit (the pre-push hook gates on `--failon error`, but warnings noise up future reviews).

- [ ] **Step 4: Verify Jenkinsfile size hasn't crossed JVM CPS limit**

Run: `wc -l /projects/elohim/genesis/orchestrator/Jenkinsfile`
Expected: `~1525` (was 1497; the new stage adds ~25 lines). The CLAUDE.md note about the 64KB JVM CPS method size limit applies to the root `Jenkinsfile`, not the orchestrator — but verify we're nowhere near. Run also: `wc -c /projects/elohim/genesis/orchestrator/Jenkinsfile` — should be well under 64KB (typically ~50KB).

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim
git add genesis/orchestrator/Jenkinsfile
git commit -m "$(cat <<'EOF'
feat(orchestrator): add Brit Plan (advisory) stage — Stage 1a wire-up

Runs brit-helper.sh verify + plan --since origin/dev as a non-blocking
advisory stage. Stage 1a behavior: warns and exits 0 if brit isn't yet
on the ci-builder image. Stage 1b (operator action: ci-builder image
rebuild) will install brit, at which point this stage produces real
advisory comparison data vs. the legacy + build-graph dispatch decision.

Stage 2 makes the brit plan authoritative for dispatch (replaces
env.PIPELINES_TO_RUN derivation) — separate plan, ships after Stage 1a/1b
stabilize for 2 weeks.

Companion to:
- genesis/docs/superpowers/specs/2026-04-27-jenkins-as-brit-attestation-producer-design.md
- genesis/docs/integrations/brit-migration-roadmap.md
EOF
)"
```

---

## Task 5: Create brit migration roadmap doc

**Files:**
- Create: `genesis/docs/integrations/brit-migration-roadmap.md`

Single-page operator + developer reference for the staged migration. Captures: what each stage delivers, what each stage requires (especially Stage 1b ci-builder image action), how to roll back at each stage, success criteria.

- [ ] **Step 1: Ensure parent directory exists**

Run: `mkdir -p /projects/elohim/genesis/docs/integrations`

- [ ] **Step 2: Write the roadmap**

Path: `/projects/elohim/genesis/docs/integrations/brit-migration-roadmap.md`

```markdown
# Brit Migration Roadmap

**Source spec:** `genesis/docs/superpowers/specs/2026-04-27-jenkins-as-brit-attestation-producer-design.md`
**Companion specs:** `elohim/brit/docs/specs/2026-04-27-build-contract-before-push-design.md`, `elohim/rakia/docs/specs/2026-04-27-rakia-as-brit-attestation-executor-design.md`

The Jenkins → brit-attestation-producer migration is staged so each stage independently delivers value and can be rolled back without disturbing later stages.

## Stage 1a — Pre-push + orchestrator advisory (LANDED — this plan)

**Plan:** `genesis/docs/superpowers/plans/2026-04-27-jenkins-stage-1a-brit-advisory.md`

**Delivered:**
- `genesis/orchestrator/scripts/brit-helper.sh` — safe-fallback wrapper
- `.husky/pre-push` — advisory `brit verify` + `brit plan --since origin/dev`
- `genesis/orchestrator/Jenkinsfile` — advisory `Brit Plan` stage

**Behavior:** warn-only everywhere. No CI behavior change. Developers gain a 5-second local plan preview.

**Rollback:** revert the three commits.

## Stage 1b — ci-builder image rebuild (OPERATOR ACTION — separate repo)

**Repo:** `ee-jenkins-ci-builder` (or wherever `harbor.ethosengine.com/ethosengine/ci-builder:latest` is sourced from).

**Steps:**
1. Add brit submodule (or pinned-version tarball download) to the ci-builder Dockerfile build steps.
2. `RUN cargo build --release -p brit-cli -p brit-build-ref` during image build.
3. Place binaries on `$PATH` (e.g., `/usr/local/bin/brit` and `/usr/local/bin/brit-build-ref`).
4. Push new ci-builder tag to Harbor (e.g., `ci-builder:1.x-brit`).
5. Update `genesis/orchestrator/Jenkinsfile` (line 702) and `genesis/Jenkinsfile` (line 123) image references to the new tag.

**Validation:** orchestrator's `Brit Plan (advisory)` stage now produces real plan output instead of `WARN: brit not installed`.

**Rollback:** revert ci-builder tag references in the two Jenkinsfiles.

## Stage 1c — Per-pipeline post.success attestation writes (separate plan)

**Plan:** `genesis/docs/superpowers/plans/2026-XX-XX-jenkins-stage-1c-attestation-writes.md` (TO WRITE after Stage 1b stabilizes)

**Scope:** add `post { success { } }` blocks to one downstream pipeline (recommend: `elohim/holochain/Jenkinsfile`, the elohim-edge pipeline) that shell `brit-helper.sh build-ref build put ...` after each successful build stage. Notes accumulate in `refs/notes/brit/builds/<pipeline>:<step>`. Push notes refs at end of pipeline.

**Behavior:** still no CI dispatch behavior change; notes accumulate as advisory data.

**Validation:** after a few builds, `git fetch origin refs/notes/brit/builds/*:refs/notes/brit/builds/*` followed by `git notes --ref=refs/notes/brit/builds/elohim-edge:cargo-build-doorway list` shows accumulating notes.

**Rollback:** revert the post-block additions; existing notes are harmless.

## Stage 2 — Consume brit plan for dispatch (separate plan)

**Plan:** `genesis/docs/superpowers/plans/2026-XX-XX-jenkins-stage-2-consume-plan.md` (TO WRITE after Stage 1c stabilizes 2 weeks)

**Scope:** orchestrator's planning stage replaced with `brit plan` shell-out; `env.PIPELINES_TO_RUN` derived from `BuildPlan.steps[].pipeline`. Downstream Jenkinsfiles' stage `when` blocks consult plan verdict instead of `params.STEPS` / `params.FORCE_BUILD`. Per-pipeline opt-in via env var for safe rollout.

**Behavior:** dispatch decisions become brit-driven. Manifest-only changes route to deploy steps narrowly (the originating-incident resolution).

**Rollback:** revert the consumption code; legacy `build-graph.groovy` still operational as fallback.

## Stage 3 — Retire legacy (separate plan)

**Plan:** `genesis/docs/superpowers/plans/2026-XX-XX-jenkins-stage-3-retire-legacy.md` (TO WRITE after Stage 2 stabilizes 2 weeks across all pipelines)

**Scope:** delete `analyzePipelineRequirements`, `runBuildGraph`, `groupByDependencyLevel`, `propagateDependencies`, `archivePipelineBaselines`, `getKnownDivergences`, the `PIPELINES` map in orchestrator (currently DEPRECATED, advisory-only). Delete `pipeline-baselines.json` artifact archival. Delete `params.STEPS`, `params.FORCE_BUILD`, `params.FORCE_DEPLOY` from downstream Jenkinsfiles. Net: ~750 lines of Groovy removed.

**Behavior:** brit is sole source of truth.

**Rollback:** revert the deletion commits.

## Operator success criteria per stage

- **Stage 1a:** `.husky/pre-push` produces `── brit advisory ──` block on every push. Orchestrator log contains `Brit Plan (advisory)` stage output.
- **Stage 1b:** orchestrator's advisory stage produces real plan JSON (not `WARN: brit not installed`).
- **Stage 1c:** `git notes --ref=refs/notes/brit/builds/<step> list` shows N entries after N successful builds of that step.
- **Stage 2:** for the originating incident scenario (manifest-only commit), `env.PIPELINES_TO_RUN = "elohim-edge"` only, total dispatch time ~90s instead of ~75min.
- **Stage 3:** `wc -l genesis/orchestrator/Jenkinsfile` shows ~750 lines (down from ~1525).
```

- [ ] **Step 3: Commit**

```bash
cd /projects/elohim
git add genesis/docs/integrations/brit-migration-roadmap.md
git commit -m "$(cat <<'EOF'
docs(integrations): brit migration roadmap (Stage 1a → 3)

Single-page operator + developer reference for the staged Jenkins →
brit-attestation-producer migration. Stage 1a (this commit's plan) is
landed; Stage 1b is operator action against ee-jenkins-ci-builder; Stages
1c, 2, 3 are separate plans gated on prior-stage stabilization.

Companion to:
- genesis/docs/superpowers/specs/2026-04-27-jenkins-as-brit-attestation-producer-design.md
- genesis/docs/superpowers/plans/2026-04-27-jenkins-stage-1a-brit-advisory.md
EOF
)"
```

---

## Task 6: End-to-end smoke test by pushing the changes

**Files:** none modified — push triggers the new pre-push hook against itself, validating the wiring.

- [ ] **Step 1: Verify all commits are local**

Run: `cd /projects/elohim && git log --oneline origin/dev..HEAD`
Expected: 4 commits — `feat(orchestrator): brit-helper.sh`, `feat(husky): brit advisory`, `feat(orchestrator): Brit Plan stage`, `docs(integrations): brit migration roadmap`.

- [ ] **Step 2: Push (which triggers the new hook)**

Run: `cd /projects/elohim && git push origin dev 2>&1 | tee /tmp/stage-1a-push.log`

Expected: existing project gates run (lint, typecheck, etc., for any changed projects), then the new `── brit advisory (Stage 1a — warn-only) ──` block appears, then push succeeds.

If the hook fails the push for any reason (e.g., a project gate caught an unrelated lint issue), fix that gate's complaint and try again. The brit_advisory function itself should never produce a failure exit code.

- [ ] **Step 3: Verify new orchestrator stage triggers and produces advisory output**

After push, watch for the orchestrator build kicked off by the webhook. Open it at https://jenkins.ethosengine.com/job/elohim-orchestrator/job/dev/ and find the new build.

Expected stages in the build view (in order): Deduplication Guard, Checkout, Plan & Decide, **Brit Plan (advisory)** ← NEW, Execute Builds, ...

In the `Brit Plan (advisory)` stage's log:
- `── brit advisory (Stage 1a — warn-only) ──`
- `[brit-helper] WARN: brit not installed; verify advisory skipped (Stage 1a)` (expected — Stage 1b hasn't run yet)
- `[brit-helper] WARN: brit not installed; plan advisory skipped (Stage 1a)`
- `── end brit advisory ──`
- `ADVISORY: legacy + build-graph dispatched: <pipeline list>`

Stage exit: SUCCESS (it's all `|| true`). Pipeline overall: same result as before Stage 1a — no behavior change.

- [ ] **Step 4: Add a follow-up TaskCreate noting Stage 1b operator action is needed**

(Manual step for the human operator running this plan: queue the ci-builder image rebuild work in your task tracker. Stage 1c plan blocks on Stage 1b completion.)

---

## Self-review checklist (run after final commit, before declaring done)

- [ ] **Spec coverage.** Re-read `genesis/docs/superpowers/specs/2026-04-27-jenkins-as-brit-attestation-producer-design.md` §3.1 (Stage 1 / orchestrator changes), §3.3 (pre-push hook integration). Confirm every Stage 1a item from the spec is covered by a task above. Stage 1b items (ci-builder image) and Stage 1c items (per-pipeline post.success blocks) are deferred per the plan's out-of-scope declaration — that's correct, not a gap.

- [ ] **Placeholder scan.** Search this plan for `TODO`, `TBD`, `FIXME`, `XXX`, `...`. Should find zero (other than this checklist item itself naming them).

- [ ] **Type / name consistency.** `brit-helper.sh` is used in: `.husky/pre-push` brit_advisory function; orchestrator Jenkinsfile `Brit Plan (advisory)` stage. Subcommand names: `verify`, `plan`, `build-ref` — consistent across all three call sites. Path: `genesis/orchestrator/scripts/brit-helper.sh` — consistent everywhere.

- [ ] **Migration doc cross-references.** `brit-migration-roadmap.md` references the spec, this plan, and future-plan filenames. Future plans don't exist yet (correct — they'll be written after each stage stabilizes).

- [ ] **Rollback procedure clear.** Each task ends with a commit. `git revert <commit>` undoes the change. Stage 1a is fully reversible without side effects (the only persistent change is potentially-ignored brit-helper.sh on disk).

If any check fails, fix inline and re-run the failing check only.

---

## Done criteria

1. All 6 tasks complete with commits.
2. Final push triggers orchestrator build that shows `Brit Plan (advisory)` stage with WARN-only output.
3. `brit-migration-roadmap.md` references this plan as "LANDED."
4. No CI dispatch behavior change observable across the next 5 webhook-triggered builds.
5. Operator queues Stage 1b (ci-builder image rebuild) as a follow-up.
