---
id: "backlog-orchestrator-timer-run-supersedes-push-run"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The orchestrator's hourly timer run (#1829, 09:00:00Z) re-dispatched edge/DNA/app on the SAME commit the 08:24Z push run (#1828) had already dispatched — the push-fired downstream builds were superseded (holochain #1431 NOT_BUILT) and the fleet roll ran twice"
slug: "orchestrator-timer-run-supersedes-push-run"
written: "2026-09-07"
author: "overnight shift 2026-09-07"
status: "open"
priority: "medium"
jobs: [elohim-orchestrator]
cluster: "ci-orchestrator-backlog"
tags: [orchestrator, over-build, timer, dispatch, museum]
---

**Evidence (2026-09-07):** push 88cd58ea9..d7bf2c265 at 08:24Z → orchestrator #1828 → elohim #1694 (SUCCESS 08:26), holochain #1431, edge #1440 (08:38). Orchestrator #1829 started at exactly 09:00:00Z (timer) while #1828 was still running and dispatched holochain #1432, edge #1441, elohim #1695 on the identical commit; #1431 ended NOT_BUILT after 35 min (superseded). Cost: a second edge build + fleet roll (~20 min churn + hours of catch-up) for zero diff. Cure: the timer trigger should no-op when the last push-fired orchestrator run for the same commit is still in flight or already dispatched that commit (compare `GIT_COMMIT` against the in-flight run's), or the timer should only dispatch pipelines whose per-pipeline baseline is stale. Museum: `2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md` (baseline-rollback over-build; NOT_BUILT ≠ regression). **Done when:** a timer run on an already-dispatched commit prints "already dispatched by #N" and triggers nothing.
