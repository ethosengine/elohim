---
id: "backlog-attention-tending-route-undeclared-at-doorway"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "POST /api/v1/attention/tending is undeclared at the doorway and has no remaining caller"
slug: "attention-tending-route-undeclared-at-doorway"
written: "2026-09-24"
author: "agent:implementer@claude-opus-5-5 (Lane A, task A9)"
status: "backlog"
priority: "low"
domain: "D2"
area: "elohim-storage build_manifest + rea-runtime attention services"
tags: [attention, tending, doorway, route-manifest, lens, lane-a]
relatedNodeIds:
  - attention-witnessed-privately
cites:
  - genesis/docs/superpowers/plans/2026-09-25-post-station-4-participant-classifier-search-attention-sprint.md
  - elohim/elohim-storage/src/api/attention.rs
  - elohim/elohim-storage/src/http.rs
  - app/elohim-library/projects/elohim-rea-runtime/src/lib/attention-tracker.service.ts
  - app/elohim-library/projects/elohim-rea-runtime/src/lib/attention-tending-api.service.ts
shift_objective: |
  Decide the future of the attention-tending write now that the content viewer no longer
  calls it. Either give it a real caller (a person deliberately tending a subject, which is
  what the AttentionTending lens kind means) and declare POST /api/v1/attention/tending in
  build_manifest() so the doorway routes it, or retire AttentionTrackerService and the route
  together. Done when the route is either declared and exercised by a scenario through the
  doorway, or removed with its services and generated intent types.
---

# The tending route: undeclared at the doorway, and no caller left

**Facts (grounded when the habit `attention-witnessed-privately` was declared).**

- `elohim/elohim-storage/src/api/attention.rs` serves `POST /api/v1/attention/tending`,
  which proxies to the `create_attention_tending` coordinator.
- The doorway routes `/api/v1/*` only by what storage's `build_manifest()` declares, and an
  unmatched path returns 404. `build_manifest()` declares nothing for attention, so a
  tending write from the browser has no route through the doorway. It works only on the Tauri
  path, which talks to the storage sidecar directly.
- Task A7 retired the only regular caller. The content viewer used to call
  `AttentionTrackerService.trackContentView` / `trackContentLeave`, a leave-time
  `AttentionTending` write that used a lens kind for dwell. Dwell and scroll depth are now a
  `lamad:content-viewed` observation (R-A1). The habit's check (3) greps the viewer for
  those calls and finds 0.

**Why this was not deleted in the sprint.** R-A1 kept `AttentionTrackerService` and the route
for actual tending: a person saying "I am tending this subject" is a deliberate act, not a
dwell measurement. The atom exists so that the decision is taken on purpose, and no one keeps a dead
route by accident.

**Options.**

1. *Declare and use.* Add `Route::post("/api/v1/attention/tending")` to `build_manifest()`
   and wire an explicit tending gesture in lamad, for example a "keep tending this" control
   on a content page. Add an a2o scenario through the doorway.
2. *Retire.* Remove the route, `AttentionTrackerService`, `AttentionTendingApiService`, the
   shefa wrapper (`app/elohim-app/src/app/shefa/services/attention-tracker.service.ts`), and
   the generated `attention-tending-intent` types in the same change, then regenerate.

Run the p2p-design-gate before option 1: tending is a lens preset, and its reach and entry type
need to be answered before it gets a route.
