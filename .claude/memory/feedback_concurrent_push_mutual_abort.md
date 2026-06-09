---
name: concurrent-push-mutual-abort
description: "Concurrent sessions pushing dev alternately abort each other's orchestrator runs (DisableConcurrentBuilds abort-previous) — one dispatcher at a time; after repeated failures GitHub may silently drop webhook deliveries entirely"
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
