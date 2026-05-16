---
id: "backlog-cleanup-scan-disposition-mechanization"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Mechanize cleanup-scan disposition for the 67-flag corpus using Run #5 taxonomy"
slug: "cleanup-scan-disposition-mechanization"
written: "2026-05-15"
author: "cartographer"
status: "envisioned"
priority: "medium"
relatedNodeIds:
  - backlog-cleanup-scan-cascade-investigation
  - backlog-cleanup-scan-disposition-taxonomy
tags: [memory-team, librarian-substrate, cleanup-scan, run6-surface]
shift_objective: |
  Take the disposition taxonomy already authored in
  `genesis/data/timeline/backlog/cleanup-scan-disposition-taxonomy.md` and walk
  the 67 active cleanup-scan flags one pass, classifying each as
  archive / dedupe / tiny-delete / hold / no-consensus per the taxonomy.
  Dispatch a librarian-judgment subagent with the taxonomy embedded; subagent
  proposes per-flag disposition with citation evidence; cartographer assembles
  the operator-confirm batch (likely 8-15 high-confidence archives, the rest
  surface as Wave-4 questions). Targets the floor for dimension #3
  (cleanup-scan flags) while harvesting over-delivery on dimension #8 (MEMORY.md
  byte size) when archives shorten the index.
---

# Body

The 67 flags have sat across three cycles without per-flag disposition; the
taxonomy backlog entry was written but not executed. This is the execution bite.
Scoped tight: one librarian-judgment dispatch, one operator review batch, one
librarian apply dispatch. Floor-clearing for dimension #3 (67 → 53 advances 14
items; the 20%-floor). Likely over-delivers because the taxonomy makes
high-confidence calls mechanical.

Acceptance: cleanup-scan flag count drops to ≤53 in next audit run; archived
entries' citations swept (cross-substrate, per the Run #5 retro); operator
surfaces any rejected dispositions for Wave 4 follow-up.
