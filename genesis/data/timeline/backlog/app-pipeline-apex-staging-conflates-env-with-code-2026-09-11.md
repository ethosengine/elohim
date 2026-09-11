---
id: "backlog-app-pipeline-apex-staging-conflates-env-with-code"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The app pipeline's result is set by apex staging (elohim.host blob forward + SSR declaration) and a SonarQube webhook-secret mismatch fires every build — five consecutive builds UNSTABLE/FAILURE with the code green, so the pipeline cannot attest code"
slug: "app-pipeline-apex-staging-conflates-env-with-code-2026-09-11"
written: "2026-09-11"
author: "shift 2026-09-11T09-00-land-batch-2c338124a (integrator) — ci-investigator reads of elohim/dev #1701–#1705"
status: "open"
priority: "medium"
tags: [jenkins, elohim-app, apex, staging, sonarqube, ci-signal, measurement-by-deploy, D8]
relatedNodeIds: []
cites:
  - Jenkinsfile
  - scripts/ci/stage-spa-blob.sh
  - scripts/ci/verify-projected-head.sh
  - genesis/data/timeline/backlog/fleet-full-arc-conductor-saturation-and-coordinated-warmup-2026-09-11.md
---

# A pipeline that measures the fleet cannot attest the code

## Observed (quoted from elohim/dev #1701–#1705)

- `#1701–#1703` UNSTABLE: `[elohim-host-landing] stage failed after 3 attempt(s) against https://elohim.host — host left STALE`, `DECLARE_ONLY: fan-out … did NOT converge` → `Setting overall build result to UNSTABLE`. `#1704/#1705` FAILURE: the same, then `SSR declaration failed on elohim.host elohim-host-landing … reason=different-serverBlobHash` → `[Pipeline] error` → Build Image / Harbor / Deploy skipped. Cause in every case: the apex doorway sheds writes (`503 catching-up, cause: upstream`) because adam's storage is starved by its saturated conductor. Zero log matches for any file the batch changed.
- Every build also prints `The incoming webhook didn't match the configured webhook secret … SonarQube quality gate check failed … Continuing pipeline...` — the SonarQube webhook secret in Jenkins and in SonarQube disagree, so the quality gate never actually gates.
- The apex retry ladder (3 attempts × artifacts + 24 × 30 s declare polls) also makes the build run 5–7× its 14-min estimate.

## Why it matters

env-red ≠ code-red is the museum's first rule, but this pipeline's `result` field encodes the fleet's mood. The orchestrator gates edge and genesis behind it, so a starved apex blocks a storage fix from deploying — the exact deploy that might relieve the apex.

## Fix (bounded)

1. Split the verdict: apex staging (`stage-spa-blob.sh`, `verify-projected-head.sh` legs against `elohim.host`) records its outcome as an ARTIFACT + junit case and marks the build UNSTABLE at most; a hard FAILURE is reserved for build/test/lint of the code. Alpha staging keeps its current teeth.
2. Fix the SonarQube webhook secret (operator-owned credential) or drop the webhook wait in favour of polling the task status, so a real quality-gate verdict can fail the build.
3. Orchestrator: do not gate edge behind an app build whose only red is the apex leg (read the artifact, not `result`).

## Done when

- An app build with green code and a starved apex ends UNSTABLE with the apex artifact attached, and edge still dispatches; the SonarQube quality gate reports a real PASS/FAIL line.
