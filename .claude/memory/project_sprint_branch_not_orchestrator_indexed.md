---
name: project_sprint_branch_not_orchestrator_indexed
description: sprint/* branches are NOT orchestrator-indexed → no CI fresh-trigger on a sprint push; CI runs on the dev-merge
metadata: 
  node_type: memory
  type: project
  originSessionId: 3385c3e4-5d32-4916-98f3-c250ae7f8923
---

The `elohim-orchestrator` Jenkins multibranch job indexes only `{PR-*, dev}` — **NOT `sprint/*`**.
This filter lives in the Jenkins multibranch SCM branch-source config (UI XML), not in any repo file,
so it's not grep-able; established via ci-investigator on 2026-05-31 (high confidence).

Consequence for an agentic `/shift` on a `sprint/*` branch: a push **delivers the webhook** (you'll
see `elohim-edge` + `elohim-genesis` create build #1 with `BranchEventCause`), but the orchestrator
never runs, so those downstream pipelines **self-skip** via their Check Trigger stage
(*"PIPELINE SKIPPED — USE ORCHESTRATOR"*, no `UpstreamCause`/`UserIdCause`) → `NOT_BUILT`, ~11s, zero
real work. **No orchestrator dispatch, no doorway/edge build, no tests run.** `[build:edge]`/`[build:all]`
commit tags are **inert** here too — they're parsed by the orchestrator, which never sees the branch.

**Why:** matches the operator-drives-sprint→dev-integration model ([[feedback_subagent_dep_conflict_supervision]]).
`sprint/*` is a local integration branch; CI is reachable only via (a) **operator merge to `dev`**
(orchestrator runs on dev → dispatches normally → deploys to alpha), or (b) a **manual `UserIdCause`
trigger** (authenticated `buildWithParameters`; with default params on `sprint/*` it builds+tests but
does NOT deploy — the edge alpha-deploy gate at `elohim/holochain/Jenkinsfile:1650` needs
`branch dev|feat-.+|claude/.+` or `FORCE_DEPLOY`/`FORCE_BUILD`).

**How to apply:** On a `sprint/*` shift, do NOT expect a CI fresh-trigger from your push and don't chase
a "missing" orchestrator build as a webhook bug — it's by design (principle-7: no drift). Treat a clean
independent LOCAL build/test as the numeric done signal and tee up the dev-merge as the operator's
cross-trigger + deploy + render-validation step. Pushing a `claude/*` branch WOULD get real CI **and
deploy to alpha** — but the edge deploy restarts the whole edge node (conductor+storage+doorway), a
heavy cluster-domain action — don't do it unattended.
