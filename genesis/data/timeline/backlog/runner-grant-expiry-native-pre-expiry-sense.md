---
id: "backlog-runner-grant-expiry-native-pre-expiry-sense"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Finite grant declarations need a native pre-expiry sense, not a test that reds dev after the window closes"
slug: "runner-grant-expiry-native-pre-expiry-sense"
written: "2026-11-09"
author: "artifact-store storage-closure shift (session f1769d1c)"
status: "open"
priority: "high"
tags: [compute-grant, runner-grant, algedonic, pre-expiry, harness-native, gate-time-bomb]
cites:
  - genesis/agentic/compute/operations.md
  - genesis/orchestrator/data/adam-compute-worker.json
  - genesis/orchestrator/compute-worker.test.mjs
shift_objective: |
  Give every finite, steward-ruled declaration (adam's G2 runner grant first) a native elohim
  pre-expiry sense: a declared measure and bound that surfaces "this grant expires in < N h, rule
  renew or withdraw" to the steward through the harness's existing channels (headline / sense /
  algedonic), well before the orchestrator gate's window test starts refusing every dev push.
  No cloud cron, no external reminder service: the operator ruled 2026-11-09 that this belongs
  in the native elohim harness.
---

## What happened

On 2026-11-09 adam's G2 runner grant (`measure-stage`, commons reach, 4 runs/h) expired at
16:00 UTC. `compute-worker.test.mjs` ("the declared Adam runner grant is inside its window, or
withdrawn") then failed inside the orchestrator gate, so every push to dev refused. The landing
lane was in use for the artifact-store storage closure at the time. The only warning before
expiry was a `t.diagnostic` line inside a passing test, printed in the last 48 h; no one reads
it there.

The steward ruled "Approve the renewal" (purpose clause
`bafyreifmazeihdvoirkdojg7vvu44gm5klphumetcyn4jzexh4wioctnp4`). The new window runs
2026-11-09T19:30Z → 2026-11-16T19:30Z, in commit `65fc3561f` on `artifact-store/e4`. The same
refusal therefore recurs on **2026-11-16T19:30Z** unless the steward rules first.

When the shift offered a cloud-routine reminder, the operator declined it: "this needs to be
part of the native elohim harness".

## What is owed

1. A declared measure (for example `grant-window-remaining@1`) per finite declaration, read from
   the declaration's own `validUntil`, with a bound (for example, refer below 48 h, breach below
   24 h), routed to the steward through the existing sense/headline surface.
2. The orchestrator test stays the floor. The sense is the early warning and must arrive before the floor
   refuses.
3. Coverage for every finite declaration the gates enforce, not only adam's: list them as part
   of the work.
4. The sense never renews anything. Renewal or withdrawal stays a steward ruling plus a reviewed
   deploy-only commit (operations.md: no automatic renewal, no timestamp advanced to pass a gate).

## Next expiry to beat

adam G2 runner grant: **2026-11-16T19:30:00Z**.
