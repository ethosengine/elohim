---
id: "backlog-orchestrator-trigger-dedup"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Remove explicit triggers{githubPush()} (webhook double-fire); reschedule cron off the late-EDT/PDT webhook window"
slug: "orchestrator-trigger-dedup"
written: "2026-06-02"
author: "cartographer"
status: "proposed"
priority: "medium"
area: "CI/orchestrator"
recurrence: 2
source_shifts:
  - "2026-05-05"
  - "2026-05-22"
domain: "code"
relatedNodeIds:
  - "memory:project_orchestrator_predictive_vision"
  - "memory:project_pre_dispatch_hard_fail_post_dispatch_unstable"
tags: [ci, orchestrator, triggers, webhook, double-fire, code-domain, recurring]
shift_objective: |
  One developer push fires two orchestrator builds: an explicit `triggers{githubPush()}`
  declaration AND the Multibranch implicit push trigger both fire, so the first build is
  immediately superseded by the second. Separately, the cron timer collides with the
  late-EDT/PDT webhook window, producing a third near-simultaneous trigger (observed 2026-05-05,
  05-22). The double/triple-fire wastes a build slot and muddies the supersede/measure logic.
  Resolve it by removing the explicit `triggers{githubPush()}` (the Multibranch implicit
  trigger is sufficient) and rescheduling the cron off the webhook window so a timed build
  doesn't collide with a push build. This is rollback-ready, code-domain config — BUT the
  trigger declaration lives in a Jenkinsfile; per the safety rule, do NOT edit any Jenkinsfile
  body in this backlog item's authoring. The implementing shift makes the minimal trigger-block
  edit + cron reschedule with rollback ready. Done when one push fires exactly one orchestrator
  build and the cron no longer collides with the webhook window.
---

# Remove the duplicate push trigger; reschedule the colliding cron

## Why this matters

Code-domain (the fix is a small trigger-block + cron edit, rollback-ready). The double-fire
isn't just wasted compute — it interacts badly with the supersede/measure logic
(`project_pre_dispatch_hard_fail_post_dispatch_unstable`), so deduping the trigger also
de-noises the orchestrator's own success measurement.

## The failure shape

- Explicit `triggers{githubPush()}` + Multibranch implicit push trigger BOTH fire on one push.
- The first build is superseded by the second (abortPrevious) → NOT_BUILT noise.
- The cron timer collides with the late-EDT/PDT webhook window → a third near-simultaneous run.

## Shape of the fix (code-domain, rollback-ready)

Remove the explicit `triggers{githubPush()}` (Multibranch implicit is enough); reschedule the
cron off the webhook window. The implementing shift makes the minimal trigger-block + cron edit
with rollback ready. (Authoring this backlog item does not touch any Jenkinsfile — root
Jenkinsfile is near the CPS cap and Jenkinsfile edits are out of scope here.)

## Acceptance

One push fires exactly one orchestrator build; the cron no longer collides with the webhook
window.
