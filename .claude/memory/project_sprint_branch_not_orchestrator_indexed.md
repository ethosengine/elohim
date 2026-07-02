---
id: project-sprint-branch-not-orchestrator-indexed
name: project_sprint_branch_not_orchestrator_indexed
title: sprint/* not orchestrator-indexed
description: 'Orchestrator indexes only {PR-*, dev}: sprint/* and claude/* pushes never trigger CI ([build:*] inert, NOT_BUILT); auto-deploy only via dev-merge.'
metadata: 
  node_type: memory
  type: project
  originSessionId: 3385c3e4-5d32-4916-98f3-c250ae7f8923
cites:
  - genesis/orchestrator/Jenkinsfile
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
cross-trigger + deploy + render-validation step.

**CORRECTION (2026-06-17, empirically retested):** a `claude/*` push does **NOT** auto-deploy either —
`{PR-*, dev}` (line 13) is the WHOLE indexed set, and `claude/*` ∉ it, so a `claude/*` push behaves
exactly like `sprint/*`: webhook delivered, orchestrator never runs, no build, `[build:*]` tags inert.
Verified this session — pushing `claude/fbootstrap-shakeout` spawned zero builds (ci-observer ×2 saw the
orchestrator multibranch still listing only `[PR-211/212/213, dev]`; operator confirmed "pipelines
quiet"), whereas the fast-forward to `dev` IMMEDIATELY spawned orchestrator #1270 → edge #1090
(UpstreamCause). The earlier "`claude/*` WOULD deploy" belief conflated TWO different things: the edge
*Deploy `when`-clause* matches `branch dev|feat-.+|claude/.+` (true — `elohim/holochain/Jenkinsfile`
Deploy Edge Node stage), but that only fires once an edge build is **triggered**, which a `claude/*`
push does not do. So the only real auto-deploy path for a feature is **land it on `dev`** (orchestrator-
indexed → edge → alpha); otherwise it needs a manual `UserIdCause` edge build (operator-only; MCP
`triggerBuild` is denied). NB: the alpha deploy restarts ALL conductor StatefulSets incl. the genesis
pair — see [[project_edge_deploy_restarts_genesis_conductors]] (benign rolling restart, rejoins same
DHT; only a force-reinstall re-keys).
