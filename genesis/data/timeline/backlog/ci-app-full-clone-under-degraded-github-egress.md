---
id: "backlog-ci-app-full-clone-under-degraded-github-egress"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "elohim (app) pipeline clones the full repository (shallow: false, 159k objects) and is the one pipeline that cannot survive a degraded GitHub fetch; genesis is dommed behind it"
slug: "ci-app-full-clone-under-degraded-github-egress"
written: "2026-10-07"
author: "release-retention-cleanup shift (Claude Fable 5.1)"
status: "backlog"
priority: "medium"
ci_status: open
fingerprints: []
jobs: [elohim, elohim-genesis, elohim-edge]
relatedNodeIds: []
tags: [ci, infra, github-egress, git-clone, shallow-clone, cascade, operator-owned, pipeline-efficiency]
cites:
  - https://jenkins.ethosengine.com/job/elohim/job/dev/1750/
  - https://jenkins.ethosengine.com/job/elohim/job/dev/1751/
  - https://jenkins.ethosengine.com/job/elohim-edge/job/dev/1566/
  - Jenkinsfile
  - elohim/holochain/Jenkinsfile
  - genesis/Jenkinsfile
---

# The app pipeline's full clone is the weak link when GitHub is slow

## Seen 2026-10-07, 01:48Z–04:10Z (alpha CI)

The fetch path Jenkins → github.com ran at 5–15 KiB/s for more than two hours. Every job
that cloned in that window died at checkout with `git-remote-https died of signal 15`,
`fetch-pack: invalid index-pack output` or `early EOF` after three retries:

- `elohim/dev` #1750 (orchestrator #1987) and #1751 (orchestrator #1988) — the app
  pipeline; both orchestrator runs ended FAILURE on this leg alone, and **elohim-genesis was
  SKIPPED-BY-UPSTREAM-FAILURE both times** (`❌ elohim failed — dooming [elohim-genesis]`),
  so the alpha seed and the household re-founding (device-recognition row 22) did not run.
- `elohim-edge/dev` #1566 — the Dataplane Validation sibling (depth 200) also died, so the
  validation that #1565 owed was never run.
- A workspace push to dev hit `send-pack: unexpected disconnect while reading sideband
  packet` once and landed on retry.

Infrastructure, not code: nothing in the tree changed between the passing and failing
clones. The network side is operator-owned (cluster egress, or GitHub).

## What the repo can do about it

The root `Jenkinsfile` (app pipeline) is the only one of the three that clones the whole
history: its Checkout stage sets `[$class: 'CloneOption', shallow: false, noTags: true]`
(159,394 objects), where `elohim/holochain/Jenkinsfile` uses `depth: 200` and
`genesis/Jenkinsfile` uses `depth: 1`. The app build reads history only as
`git log -1 --format=%B` (the commit-tag scan) and `git log -1 --format=%ct` (the build
stamp), both satisfied by a depth-1 clone. A shallow clone is 50–100× fewer bytes, which is
the difference between surviving and dying at 10 KiB/s, and it is what the sibling pipelines
already do.

Two further shapes worth weighing in the same change:

1. **The cascade.** genesis is doomed whenever the app build fails for any reason. A
   checkout failure is not an app failure; the orchestrator could distinguish "the pipeline
   could not start" (NOT_BUILT-class, retry) from "the build failed" (cascade), so a network
   stall does not also cancel the seed. The anti-patterns museum already names
   NOT_BUILT/superseded ≠ regression for the reading side; this is the dispatch side.
2. **A repository mirror** inside the cluster (or Jenkins' reference-repo clone option)
   would make every pipeline's checkout a local fetch; that is the operator's call and the
   durable fix.

## Current decision

Not changed in the shift that found it: the root `Jenkinsfile` was outside the shift's path
scope, and the pipelines were in flight. The shallow-clone change is a one-line edit with the
two `git log -1` reads as the only history consumers to verify; the cascade distinction and the
mirror are design questions for the orchestrator's owner.

## Probe

The next app build on dev after the change shows `--depth=…` in its `git fetch` line and
completes Checkout in under a minute; a later slow-egress window (fetch under 50 KiB/s)
leaves genesis dispatched rather than SKIPPED-BY-UPSTREAM-FAILURE.
