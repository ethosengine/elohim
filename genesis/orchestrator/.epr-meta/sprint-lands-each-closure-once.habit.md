---
epr-habit-version: 1
id: sprint-lands-each-closure-once
invariant: >
  Within one steward's grant: a closure is gated once across his devices; a backlog item is
  claimed by one actor; a handoff carries bytes, never prose; dev-integration is held only for the
  CAS window; a dormant author never blocks the queue; every dev tip names the landing intents it
  carries; the one Jenkins verdict on that tip reaches every device's fold; no change to the gate is
  judged by its own candidate bytes; and a said obligation reaches an actor who can act on it, or its
  sender learns by exit code that it did not.
status: red
active: false
checks:
  - "a2o @concern:sprint-lands-each-closure-once (genesis/a2o/features/devflow/landing-queue.feature, @act:host): duplicate claim refused naming holder; offer fetched by oid; sibling green reused and labelled independence:sibling; intent queues, never refused; dormant head skipped and the next lands within dormant_after_s+600s; CAS lease < 120 s; edge-selecting train waits on alpha-fleet during an edge wave; governance-class change judged by base recipe; red with no local repro reverts nothing; note to lane:X reaches its current holder"
  - "the same tag and @requires:shem (joined real-workspace receipt)"
  - "node --test genesis/orchestrator/gate-evidence.test.mjs (sibling-signer cases)"
  - "per-sprint bounds over gate-cycle-seconds@1 + delivery-series.mjs: duplicate gated closures = 0, duplicate claims = 0, lease p90 < 120 s, >= 2 intents per Jenkins wave (unwired until A0)"
refs:
  - "spec: genesis/docs/superpowers/specs/2026-10-27-steward-space-landing-plane-design.md (§2 Arm A slices A0-A13; §3 this habit and the WIP fence)"
  - "upstream of: genesis/orchestrator/.epr-meta/push-delivers-within-budget.habit.md (active red; A9/A10 move its measured ratio)"
  - "sibling: genesis/agentic/.epr-meta/devices-share-one-berth.habit.md (channel reliability stations)"
retire-when: >
  landing intents, verdicts and change-class acceptance are served by brit-native integrate over the
  dataplane in a collective space (S2), so this repo-local queue adds no information; or it retires
  together with push-delivers-within-budget.
---
DELTA 2026-10-27 (BORN red, inactive): nothing in the invariant is kept yet. The 2026-10-27 overnight evidence: both devices fixed the same eprfs reds (no claim crossed), a fix crossed as prose and was re-typed, shem's push held dev-integration for 3 h+ and was refused three times, and the orchestrator ran 8 consecutive FAILUREs at 2.4-4.7 h each. landing-queue.feature is not written. Inactive because the WIP fence is full; it is the upstream station of push-delivers-within-budget.
