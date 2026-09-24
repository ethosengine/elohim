---
id: "backlog-lifestream-associative-and-spatial-views"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Lifestream recompositions beyond time — associative and spatial views over the person's own observations"
slug: "lifestream-associative-and-spatial-views"
written: "2026-09-24"
author: "agent:implementer@claude-opus-5-5 (Lane A, task A9)"
status: "envisioned"
priority: "low"
domain: "D2"
area: "elohim-storage observation views + lamad /me"
tags: [observation, lifestream, recipe, recomposition, attention, lamad, lane-a]
relatedNodeIds:
  - attention-witnessed-privately
cites:
  - genesis/docs/superpowers/plans/2026-09-25-post-station-4-participant-classifier-search-attention-sprint.md
  - elohim/elohim-storage/.epr-meta/elohim/algorithms/observation-lifestream-recipe.json
  - elohim/sdk/schemas/v1/views/observation-stream-view.schema.json
  - app/lamad/src/app/components/my-stream/my-stream.component.ts
shift_objective: |
  Add two more recompositions of a person's own observations, each rendered through its own
  governed recipe whose CID the page prints, in the same shape as the lifestream: an
  associative view (what the person read, grouped by the content graph's relationships
  between the subjects) and a spatial view (their reading placed on the meaning map). Done
  when each has a recipe JSON beside observation-lifestream-recipe.json, a Category C view
  schema, a GET route over the requester's own rows only, and a /lamad/me page that prints
  provenance the way /lamad/me/stream does.
---

# Associative and spatial views of the lifestream

**What landed.** Ruling R-A4 built one recomposition first: the lifestream, time-ranked then
dwell-ranked, served by `GET /api/v1/observations/stream` over the requester's own rows and
rendered through `observation-lifestream-recipe.json` (audience `self`, default window 7d,
lens table `all | content | long-dwell`). `/lamad/me/stream` prints the recipe name and CID
before any entry.

**What R-A4 captured and deferred.** The same private rows answer other questions, and each
answer is a different recipe:

- **Associative.** "What have I been circling?" Group the viewed subjects by the content
  graph's typed relationships (prerequisite, teaches, references; `ContentGraphResolver`),
  and rank each cluster by combined dwell.
- **Spatial.** "Where have I been?" Place the viewed subjects on the meaning map
  (`/lamad/map`) with dwell as weight. This view is personal, so it is never a heat map of
  other people.

**The shape to keep** (the reason these are cheap once the lifestream exists):

1. A recipe file in `elohim/elohim-storage/.epr-meta/elohim/algorithms/` with its purpose,
   audience `self`, ranking, window, lens table and omissions policy. Its CID is computed
   the same way (`blake3:`), and the page prints it.
2. A view schema in `elohim/sdk/schemas/v1/views/` (Category C, rendered per request, never
   persisted), a Rust struct, and a codegen entry.
3. A route that reads only rows whose observer is the header, with no fallback to other
   observers.
4. Named omissions: anything the view cannot show or vouch for is a line, never a silent gap.

**Why this matters.** Lifestreams, Nepomuk and Jenson's recomposed views were all
recompositions of a person's own traces. What the protocol adds is that each arrangement is
a governed, printed recipe the person can contest, and the traces never leave their node.
Two more views would show that the pattern generalises beyond time order.
