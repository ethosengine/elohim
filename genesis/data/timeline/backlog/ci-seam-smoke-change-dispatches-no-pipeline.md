---
id: "backlog-ci-seam-smoke-change-dispatches-no-pipeline"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A change to scripts/ci/substrate-seam-smoke.sh dispatches no pipeline — the script the edge Dataplane Validation stage runs reaches CI only by a forced tag"
slug: "ci-seam-smoke-change-dispatches-no-pipeline"
written: "2026-10-10"
author: "steward-exile-probe shift, 2026-10-10"
status: "open"
priority: "medium"
jobs: [elohim-orchestrator, elohim-edge]
tags: [ci, change-detection, under-build, seam-smoke, graph-walker]
---

**Measured.** `git diff --name-only --cached | node genesis/orchestrator/graph-walker.mjs` answered
`{"projects":[],"pipelines":[]}` for dev 446b72c37, whose whole diff is `scripts/ci/substrate-seam-smoke.sh` and its
test. `elohim/holochain/Jenkinsfile runDataplaneValidation` runs that script on every edge validation. The new leg
reached a console (edge #1594) only because the commit carried a forced validate-only edge tag.

**Why it matters.** A regression in the seam smoke would land on dev with no run, and show up on the next unrelated
edge build as that build's failure.

**Bounded task.** Declare `scripts/ci/substrate-seam-smoke.sh` (and the other `scripts/ci/*.sh` the edge validation
stage calls) as inputs of the edge project in `elohim/holochain/build-manifest.json`, so a change selects a
validate-only edge run rather than nothing or a full roll. Check the walker's answer for the same diff afterwards.

**Related, owned elsewhere.** While `operator-answers-pain.feature` and `runtime-band-external-imposition.feature`
stay born red without `@wip` (eeb77c5de, `pain-is-answered`), every edge validate-only run ends FAILURE — #1591 to
#1594 — and turns its orchestrator run red with it (#2021). A new red in that stage cannot be told from the standing
one by build result alone; read the scenario counts.
