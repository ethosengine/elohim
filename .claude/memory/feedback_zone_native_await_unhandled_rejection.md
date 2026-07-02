---
id: feedback-zone-native-await-unhandled-rejection
name: zone-native-await-unhandled-rejection
title: zone.js native-await phantom uncaught
description: zone.js checks uncaught rejections at drain-end, before native await's V8 thenable-job attaches — handled rejections false-flag; fix with sync .then/.catch.
metadata:
  node_type: memory
  type: feedback
cites:
  - app/elohim-app/src/test-setup.ts
---

# zone.js + native async/await: phantom "Uncaught (in promise)" on handled rejections

In any zone-based Angular spec (vitest/analogjs setup-zone), a mocked rejection
(`mockRejectedValue`, or any rejection delivered *within* a zone microtask drain)
that flows into component code consumed by **native `await`** gets false-flagged
as an Uncaught Exception — even though a try/catch or `.catch` demonstrably
consumes it. Under full-suite load the phantom error is attributed to whichever
tests happen to be running (nondeterministic 1-2 test failures in unrelated files).

**Mechanism (zone.js 0.15, verified by reading fesm2015/zone.js):**
ZoneAwarePromise runs ALL its handler microtasks from one internal queue drained
in a single native job, then checks `_uncaughtPromiseErrors` at drain-end
(`microtaskDrainDone`). A rejected promise with an empty subscriber queue is
pushed to that list immediately; a later `.then` attach rescues it
(`clearRejectedNoCatch`) — but native `await` attaches via a separate V8
thenable-job that runs AFTER zone's drain, so the rescue comes too late. A
`Promise.all` aggregate whose members are already-rejected (or reject inside the
drain, e.g. via queueMicrotask) always trips this.

**Why:** The false positive is invisible in isolation (tests still pass; vitest
reports "1 unhandled error") but fails innocent tests under load — it presents
as flaky cross-file failures that no amount of test-local debugging explains.

**How to apply:**
- Fix at the COMPONENT: chain `.then/.catch/.finally` instead of `await` where a
  promise may reject within a microtask drain — synchronous handler attachment
  closes the race for every caller. Worked example: `loadGovernanceViews` in
  app/lamad/src/app/components/content-viewer/content-viewer.component.ts.
- Do NOT bother with: pre-attaching `.catch` to the mock's promise (the flagged
  promise is the derived aggregate, not the mock), switching fakeAsync→async
  (drain mechanism identical), or queueMicrotask-deferred rejections (same drain).
  A setTimeout-deferred rejection escapes (macrotask), but the component-side fix
  is the durable one.
- Diagnosis signature: vitest "Uncaught Exception" whose serialized error carries
  `__zone_symbol__currentTaskTrace` with `scheduleResolveOrReject` in the stack,
  error construction pointing at the mock's `new Error(...)` line.

Related: [[lint-autofix-string-scan-poison]] (sibling "test-infra poisons CI" class).
