---
id: "backlog-content-view-economic-event-retires-into-summary-graduation"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Retire the per-view EconomicEvent emit into the observation layer's summary graduation (spec §8.2)"
slug: "content-view-economic-event-retires-into-summary-graduation"
written: "2026-09-24"
author: "agent:implementer@claude-opus-5-5 (Lane A, task A9)"
status: "backlog"
priority: "medium"
domain: "D2"
area: "lamad signal harness + elohim-storage observation evaluator"
tags: [observation, economic-event, graduation, attention, signal-harness, rea, lane-a]
relatedNodeIds:
  - attention-witnessed-privately
cites:
  - genesis/docs/superpowers/plans/2026-09-25-post-station-4-participant-classifier-search-attention-sprint.md
  - genesis/docs/content/elohim-protocol/architecture/2026-05-11-observation-event-layer-design.md
  - app/lamad/src/app/services/signal-harness.service.ts
  - app/lamad/src/app/components/content-viewer/content-viewer.component.ts
shift_objective: |
  Stop emitting one EconomicEvent per content view from the browser's signal harness. Content
  views are already witnessed as agent-private lamad:content-viewed observations; what the
  commons needs from them (that a contributor's work was read) should graduate as a windowed
  summary EconomicEvent per the observation spec §8.2, produced on the node from observations,
  carrying counts and a diversity summary, never the reader's identity trail. Done when the
  content viewer no longer calls signalHarness.onRendererComplete for the view emit, a summary
  evaluator emits one EconomicEvent per window per subject from observation rows, and no
  per-reader view event leaves the node.
---

# The per-view EconomicEvent retires into summary graduation

**What exists.** The content viewer does two things when a page loads. It opens a witnessed
view (`ObservationEmitterService.begin`, task A7), and it also calls
`signalHarness.onRendererComplete(contentNode, …)`. That second call is a manifest-driven
`onConsume` signal, and it emits an `EconomicEvent` for the view. Ruling R-A1 kept the
emit for the sprint on purpose, so that the lens-kind misuse (the tending write on leave) was
retired first and nothing else changed at the same time.

**Why it must go.** An EconomicEvent per view per reader is the same trace the attention
habit keeps private, just published under a different name. `features/lms/attention-analytics.feature` described
this earlier framing ("content interactions recorded as economic events"). Its dashboard
scenario is now superseded by `attention-witnessed-privately.feature`.

**Where it goes.** The observation spec §8.2 ("Observation → summary EconomicEvent") covers
this case. The manifest declares a kind with `graduation_policy: "summarize"`. An evaluator
closes a window and emits one EconomicEvent with `observation_refs` and a
`diversity_summary` in place of N per-event entries. Content attention fits that shape:
"subject X was read N times this week by M distinct readers" is value flowing to a
contributor, and nobody's individual reading becomes public.

**Open questions for pickup.**

- `lamad:content-viewed` is `reach: agent-private`. A summary over private rows needs a
  consent line: either a per-person opt-in to be counted, or a count that cannot be traced
  back to anyone (a floor on M before a summary is emitted). Run the p2p-design-gate on the
  summary's reach.
- Which node evaluates the window? Only the reader's own node holds their rows, so the
  summary is each node's contribution. A cross-node fold is an aggregation seam, and it is
  not the doorway's job.
- The EconomicEvent entry type already exists, so no new DHT entry type is needed. The new
  action verb is declared in the manifest.
