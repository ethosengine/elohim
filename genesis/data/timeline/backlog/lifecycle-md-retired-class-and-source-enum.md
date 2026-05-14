---
id: "backlog-lifecycle-md-retired-class-and-source-enum"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "LIFECYCLE.md retired-class semantics + delivery_status_source enum growth"
slug: "lifecycle-md-retired-class-and-source-enum"
written: "2026-05-14"
author: "cartographer"
status: "refined"
priority: "medium"
relatedNodeIds:
  - "memory:feedback_story_delivery_status_axis"
  - "memory:feedback_inline_summary_must_echo_frontmatter"
  - "backlog:persona-rename-canonical-flip"
  - "chronicle:2026-05-14-memory-ceremony-run-2"
tags: [lifecycle, conventions, delivery-status, retired-class, audit-script, tightening]
shift_objective: |
  Tighten the author/delivery axis split in two small ways the Run #2 ceremony
  surfaced but didn't canonize. (1) Document retired-class semantics:
  retired stories (status: retired) do NOT track delivery_status by mirror and
  do NOT freeze at retirement — they remain delivery_status: unknown with an
  explicit retired-delivery-axis-na source annotation. (2) Grow the
  delivery_status_source enum to cover the patterns that emerged in practice
  during Run #2 and the Wave 0 baseline (bootstrap-<date>, triage-<date>-<roles>,
  retired-delivery-axis-na, pending-deliver-judgment, /deliver-<verdict-iteration>).
  Update LIFECYCLE.md "author/delivery axis split" matrix with the retired-class
  row; update genesis/data/stories/CONVENTIONS.md delivery_status_source
  field-semantics with the enumerated pattern list; update
  delivery-status-distribution.py KNOWN_SOURCES so Wave 0 baselines regenerate
  with zero source warnings. Small-scope tightening; should fit one short
  shift.
---

# LIFECYCLE.md retired-class semantics + delivery_status_source enum growth

## Why this matters

The Run #2 ceremony introduced the author/delivery axis split as a load-bearing
convention. Two small but real gaps remain in how that convention is documented.
First, retired stories don't fit the gradient cleanly — they have no delivery
mirror because the story itself is no longer the narrative we tell, so polling
for `/deliver` evidence is meaningless. The Run #2 ceremony resolved this in
spirit (the librarian held retired entries at `delivery_status: unknown` rather
than freezing them at a frozen-in-time gradient value), but the convention isn't
written down — the next ceremony will re-derive it under different pressure.
Second, `delivery_status_source` is enumerated in CONVENTIONS.md as
`deliver-bridge | deliver-bridge-floor | operator-override`, but real entries
written during the ceremony used richer patterns (`bootstrap-2026-05-14`,
`triage-2026-05-14-cartographer-librarian`, `pending-deliver-judgment`) that the
audit script either ignores or flags as anomalies. Both gaps are tiny in scope
but compound the longer they sit — every future Wave 0 baseline either regenerates
warnings or drifts the spec to accommodate ad-hoc additions.

## What's blocking

Nothing substrate-side. The conventions need an editor pass; the audit script
needs five constant strings added to its `KNOWN_SOURCES` (or wherever the source
patterns are tolerated). Operator approval to consolidate the two gaps into one
backlog entry is the only gate — done by virtue of this entry existing.

## What's ready

- The Run #2 chronicle (`chronicle:2026-05-14-memory-ceremony-run-2`) documents
  the four-way convergence that named the axis split; the discipline is grounded
- LIFECYCLE.md "author/delivery axis split" section (line 164+) already carries
  the unified gradient + disposition matrix; the retired-class row slots in as a
  matrix extension, not a new section
- `genesis/data/stories/CONVENTIONS.md` line 131 already enumerates
  `delivery_status_source` values; the enum just needs to grow
- `delivery-status-distribution.py` already reads `delivery_status_source` from
  frontmatter (line 202, 240); a `KNOWN_SOURCES` set + tolerance for the
  documented patterns is one edit
- The patterns to enumerate emerged organically and are already in use in
  ceremony-written frontmatter — this is documentation catching up to practice,
  not new design

## Convergence

The persona-rename-canonical-flip backlog (the upstream precedent for "operator
flips one bit; conventions need to absorb it") established that small
conventions-side tightenings should follow ceremony surfacing rather than wait
for a full sprint. Same shape here: Run #2 surfaced the gap, the next ceremony
will hit it again if we don't write it down. The two feedback memory entries
(`feedback_story_delivery_status_axis` and
`feedback_inline_summary_must_echo_frontmatter`) name the discipline; this
backlog converts wisdom into convention text + audit-script behavior so the
discipline is mechanically enforced.

## Definition of done

1. **LIFECYCLE.md "The author/delivery axis split" section** gains a
   retired-class disposition row in the matrix at line ~208 (Disposition
   matrix — extended with delivery axis). Row text: retired stories
   (`status: retired`) carry `delivery_status: unknown` +
   `delivery_status_source: retired-delivery-axis-na`; no `/deliver` polling;
   no freeze-at-retirement (the gradient mirrors live substrate, not historical
   substrate, and retired stories are no longer the substrate we mirror).

2. **`genesis/data/stories/CONVENTIONS.md` `delivery_status:` field-semantics
   block** (around line 128-131) gains an explicit note: "Retired stories
   (`status: retired`) use `delivery_status: unknown` with
   `delivery_status_source: retired-delivery-axis-na`. They are not polled by
   `deliver-bridge` and are not frozen at a prior gradient value."

3. **`delivery_status_source` accepted patterns are explicitly enumerated**
   in both LIFECYCLE.md and stories/CONVENTIONS.md (or in a shared
   conventions block referenced by both):
   - `deliver-bridge` — verdict from `/deliver`
   - `deliver-bridge-floor` — only a2o floor signal, no verdict yet
   - `operator-override` — rare; backfilling
   - `bootstrap-<YYYY-MM-DD>` — initial baseline write during a ceremony
   - `triage-<YYYY-MM-DD>-<roles>` — multi-agent triage write (e.g.
     `triage-2026-05-14-cartographer-librarian`)
   - `retired-delivery-axis-na` — story status is retired; axis does not apply
   - `pending-deliver-judgment` — feature exists, `/deliver` has not yet judged
   - `/deliver-<verdict>-<iter>` — direct write from a `/deliver` iteration
     (e.g. `/deliver-delivered-iter3`)

4. **`delivery-status-distribution.py` `KNOWN_SOURCES` set updated** to
   validate (or at minimum, accept without warning) the patterns above. Pattern
   matching for date- and role-suffixed forms uses a small prefix-and-shape
   check, not full regex (keep stdlib-only per `_lib` discipline).

5. **Run #3 ceremony Wave 0 baseline regenerates cleanly** with zero
   `delivery_status_source` warnings against any existing story or backlog
   entry; any new warning at Run #3 must point to a frontmatter typo or a
   genuinely unrecognized pattern, never to a documented one.
