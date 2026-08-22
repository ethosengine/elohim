---
index: false
name: concurrent-push-mutual-abort
title: Concurrent-push mutual abort
description: "Dev pushes minutes apart kill each other's builds (abort-previous), even same-session; one push per batch, wait until COMPLETE; escalate silent webhook loss."
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 2c7ad11d-dd51-41df-8b9b-eea659f95321
---

Observed 2026-06-06 evening: five consecutive `elohim-orchestrator/dev` ABORTED runs (#1172–#1176, "Queue task was cancelled") and two push events that never spawned a run at all. Two causes layered:

1. **Mutual abort**: the orchestrator job carries `DisableConcurrentBuildsJobProperty` (abort-previous). Two active sessions pushing `dev` minutes apart — retriggers, sentinel commits, `[build:*]` nudges — each new BranchEventCause killed the predecessor before it could dispatch downstream. Nobody's intent ever shipped.
2. **Silent webhook loss**: after the abort storm + a transient storage-503/harbor-500 window, later push events produced NO queue item (queue empty, agents online, job buildable, nextBuildNumber unchanged) — consistent with GitHub auto-muting the hook after repeated delivery failures. More pushes cannot fix this; it needs operator-side redelivery / "Scan Multibranch Now" / a manual UserIdCause build.

**Why:** push-as-trigger is a shared, last-writer-kills-previous channel; "carrying queued intent" in a commit message only works if that event actually lands AND survives.

**How to apply:** when CI dispatch matters, coordinate to ONE dispatcher between concurrent sessions ([[concurrent-sessions-shared-worktree]]); after any abort storm, verify a run actually SPAWNED (lastBuild number advanced) before assuming the event queued; if events go silent (no queue item, nextBuildNumber frozen), stop pushing and escalate to the operator for hook redelivery or manual trigger.

**SAME-session sequential pushes cascade-abort too (re-confirmed 2026-06-27, cost a deploy + rework).** This is NOT only a two-session hazard. One session making N sequential `dev` pushes minutes apart = N−1 aborted DOWNSTREAM builds: each push spawns an orchestrator that dispatches a downstream build (e.g. app `elohim/dev`), and the NEXT push's orchestrator kills BOTH the prior orchestrator AND its already-running downstream job via abort-previous — observed `elohim/dev #1564` (ABORTED mid-unit-test) and `#1565` (ABORTED mid-stage), only `#1566` (the last push) survived to deploy. **"Verify a run SPAWNED" is INSUFFICIENT as an abort-safety gate** — a spawned, actively-building downstream job is still killed by the next push (it was 6.8 min into its run). The orchestrator runs themselves go NOT_BUILT ("Build(s) failed: elohim, elohim-edge - Aborting") because their downstream was cancelled. **The only safe rule: ONE push per logical batch — bundle everything that must build together into a single push, then wait for the whole run to COMPLETE (not merely spawn) before the next push.** If you've split work into staged pushes, the deploy/verify of all but the last gets thrown away. Recovery from a stranded deploy: an empty `[build:app]` commit as the FINAL push (no further pushes) lets the app build run uninterrupted.
