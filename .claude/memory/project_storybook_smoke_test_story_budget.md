---
name: project_storybook_smoke_test_story_budget
title: Storybook smoke-test story-budget
description: "Smoke-Test Stories stage shares a cumulative timeout budget; a fat story matrix fails the WHOLE build — keep Library A+B to ~3-4 stories on trimmed fixtures."
metadata: 
  node_type: memory
  type: project
  originSessionId: ca4a672a-d664-44dd-b21f-64d780015b5d
---

The `elohim-storybook` pipeline runs a **Smoke-Test Stories** stage that loads every story with a per-story timeout (~30s) under a marginal total budget. A fat per-element story matrix — many `lens×theme×state` exports, or heavy-DOM full-fixture stories, or stories with NO `play` function that stall the runner — tips the cumulative budget and fails the **entire** build (UNSTABLE/FAILURE), not one story.

Validated on the seam-map work: #178 FAILED (13 default `<elohim-seam-map>` stories piled onto pre-existing slow qahal `power-user-view`/`simple-user-view` stories → 272s stall) → #179 SUCCESS after trimming to 4 → #180 SUCCESS with 3 *designed* (Library B) stories all on a trimmed 6×5 fixture.

**Why:** the gate's budget is shared across all stories and the pipeline runs near its ceiling; adding heavy stories (or ones that stall) pushes a previously-green build over. Trimming YOUR new stories is the lever you control to get back under budget.

**How to apply:** keep BOTH Library A (`.default.stories.ts`) and Library B (`.designed.stories.ts`) sets to the core contract (~3–4 stories). For matrix-heavy elements, use a trimmed fixture (`slice` devices/seams/routing — see the seam-map stories) so each story is light DOM. Don't add CustomTheme/Unstyled/full-matrix variants to Library B (those are Library A's job). And remember: **CI-green ≠ binding-correct** — a dead CSS-custom-prop binding still "renders," so verify brand bindings with eyes on the LIVE render, light theme as the detector. See [[feedback_frontend_review_eyes_first]] and [[project_graphos_dead_binding_classes]].
