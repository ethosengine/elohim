---
epr-habit-version: 1
id: idle-is-free
invariant: >
  A node at rest is quiet. With nothing being authored, read or repaired, a storage peer asks
  its conductor for almost nothing and the conductors leave the processor alone: background
  work is driven by what changed, remembers what it has already settled, and backs off from
  what keeps failing, so the cost of a quiet hour does not depend on how much history the node
  holds. Work done on behalf of nobody is a defect, whatever it was meant to keep in order.
status: red
active: false
checks:
  - "a2o @concern:idle-is-free (genesis/a2o/features/dataplane/household-at-rest.feature — @act:i, the household lane: `just test mesh features/dataplane/household-at-rest.feature`). Two snapshots 300 s apart on a settled household with nobody acting: each storage peer's conductor calls (admission permits released) at most 6 a minute, no permit refused, and the three conductors together at most 10 CPU seconds a minute. The budgets are the target, not a reading."
refs:
  - "genesis/data/timeline/backlog/conductor-cap-grant-scan-per-zome-call.md — what unbounded per-call cost did to the fleet"
  - "genesis/data/timeline/backlog/conductor-admission-saturated-for-hours-after-restart.md"
  - "genesis/data/timeline/backlog/projection-reconcile-actionable-sawtooth.md — the same ~50 items rediscovered every 20–30 minutes and never healed"
  - "elohim/holochain/.epr-meta/zome-call-cost-bounded.habit.md — the sibling: what ONE call costs; this habit is how MANY calls nobody asked for"
retire-when: >
  when every background loop in the node declares its trigger (an event, a change set or a
  bounded timer) and its settled-state memory by construction — so that a loop which re-asks
  what it already knows cannot be written — and a running node reports its own idle cost
  against a declared ceiling, making a separate household rehearsal redundant.
---
DELTA 2026-09-19 (declared RED with its check): first reading, household of three holding 27 pieces of content, settled 5 min, nobody acting, 10-minute window (genesis/a2o/reports/recovery/serving-edge-20260919/idle-baseline.log): conductor calls a minute — matthew 49.7, jessica 49.3, james 74.0 (budget 6); permits refused 0; conductor CPU 87.6 s/min — about one and a half cores pinned — storage 13.3, doorway 0.9 (budget 10). Nearly all calls are `content_store`. The storage log over the same window names the producers: projection reconcile "head unchanged — refreshed row" ×79, the same STATE-DIVERGENT commitment re-reported ×124, `apply_delta failed — page held for retry` ×63, and "serving local inventory" ×270 — reconciliation that re-derives a settled world each sweep. Born of the alpha stall, where all seven conductors sat at their CPU limit while serving no one.
