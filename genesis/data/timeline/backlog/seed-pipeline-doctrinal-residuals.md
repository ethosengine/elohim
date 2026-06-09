---
id: "backlog-seed-pipeline-doctrinal-residuals"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Seed-pipeline residuals from the doctrinal corpus: relative ./sibling.md links don't EPR-translate; meta/index.json misses the three new docs"
slug: "seed-pipeline-doctrinal-residuals"
written: "2026-06-09"
author: "claude-fable-graph-seam-delivery"
status: "documented"
priority: "low"
ci_status: documented
jobs: []
relatedNodeIds: []
tags: [content-pipeline, elohim-import, seed-data, doctrinal-corpus, epr-links, low-priority]
cites:
  - genesis/data/lamad/content/confession.json
  - genesis/data/lamad/meta/index.json
  - app/elohim-app/src/app/app.component.ts
---

# Seed-pipeline residuals: doctrinal corpus link translation + catalog registration

Two small residuals surfaced during the native-content-graph-seam delivery audit
(2026-06-09). Neither blocks the sidebar/discovery experience.

## 1. Relative `./sibling.md` links in doctrinal content don't EPR-translate

The four doctrinal docs cross-reference each other with relative markdown links
(`[the disputation](./theology.md)`). These survive verbatim into the seed JSON
`content` (pre-existing in the old snapshot too — NOT introduced by the 2026-06-09
md→json re-export) and render in-app as relative hrefs that resolve to nothing.
The epr-link capture-phase interceptor is the safety net for content-authored
anchors, but `./theology.md` is not an EPR-shaped target it can claim.

**Proposed fix:** at import/re-export time, translate intra-corpus relative md
links to `/epr/{slug}` (or `epr:{slug}`) — a small transform in the elohim-import
CLI or the re-export script. Verify with a link-click in the markdown renderer.

## 2. `genesis/data/lamad/meta/index.json` misses constitution/confession/theology

Only `manifesto` of the four is registered in the generated catalog (3,495 nodes).
The seeder explicitly skips `index.json` and no runtime consumer was found, so
this is a latent inconsistency, not a defect — but any future catalog/MCP tool
reading it will under-count the doctrinal corpus. Regenerate the catalog (or
confirm it's dead and retire it).

## 3. Path-phase step relationships rejected: `relationship_type 'step'` not in storage vocabulary

The seeder's path phase (`--paths-only`) emits step relationship rows with
`relationshipType: 'step'`; storage's validator rejects the whole bulk batch:
`HTTP 400 — relationship_type 'step' is not valid. Valid types: ["RELATES_TO",
"CONTAINS", ...16 kinds]`. Same relationship-*kind* vocabulary-drift family the
graph-seam spec §2 deliberately sidestepped (manifest 11 / DHT 6 / storage 16).
Path STEP membership still works (it lives in the path ContentNode's body
`sections[].items[]`), so this only drops the redundant step-edge projection —
but the bulk-batch all-or-nothing rejection means any OTHER relationships in the
same batch are lost with it. Fix: either map `step` → `CONTAINS` at the seeder,
or admit `step` to storage's vocabulary as part of the tracked kind-drift
reconciliation.

## Status

`documented` — not actioned. Captured during the graph-seam delivery shift so the
discoveries aren't orphaned.
