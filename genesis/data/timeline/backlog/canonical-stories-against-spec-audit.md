---
id: "backlog-canonical-stories-against-spec-audit"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Author canonical-stories-against-spec audit script to surface spec-without-story coverage gap"
slug: "canonical-stories-against-spec-audit"
written: "2026-05-14"
author: "cartographer"
status: "backlog"
priority: "medium"
relatedNodeIds:
  - "memkit:2026-05-14/story-coverage-audit"
  - "memory:project_wisdom_resolves_into_epics"
  - "memory:project_three_temporal_perspectives"
  - "memory:feedback_story_delivery_status_axis"
  - "backlog:storyteller-coverage-sprint"
  - "chronicle:2026-05-14-memory-ceremony-run-2"
tags: [audit, storyteller, coverage, spec-without-story, deterministic-substrate]
shift_objective: |
  story-coverage-audit.py answers "which features on disk lack a canonical story?" It does
  not answer the sibling question: "which specs in genesis/docs/superpowers/specs/ have
  shipped (or are shipping) without a canonical story documenting the human experience?"
  Specs are the design layer; features are the executable layer; stories are the narrative
  layer. The current audit measures the gap between features and stories; the gap between
  specs and stories is currently unmeasured and accumulates silently as specs ship faster
  than the storyteller authors.

  Author `canonical-stories-against-spec-audit.py` mirroring the existing audit's shape:
  - Inventory specs at `genesis/docs/superpowers/specs/*.md` (and plans at
    `genesis/docs/plans/` if they declare deliverables)
  - Cross-reference against story frontmatter (`sourced_from.spec_refs[]` once that
    schema lands, or full-text match on spec slug until then)
  - Output `.claude/memory-kit/<date>/spec-coverage-audit.md` ranking specs by
    "shipped-without-story" risk (signal: spec has a related feature file with passing
    scenarios but no canonical story anchoring the narrative)
  - Wire into librarian Wave 1 prologue alongside story-coverage-audit per the
    ceremony substrate pattern

  Done when (a) script exists at `.claude/scripts/memory-kit/spec-coverage-audit.py` and
  runs cleanly; (b) deterministic markdown output at the expected path; (c) librarian's
  Wave 1 invokes it and surfaces the data; (d) at least one ceremony's cartographer reads
  it as Wave 1 substrate before this entry retires.
---

# Canonical-stories-against-spec audit

## Why this matters

The substrate has three layers of truth in the narrative-coverage problem:

| Layer | Artifact | Existing audit |
|---|---|---|
| Design | `genesis/docs/superpowers/specs/*.md` | NONE |
| Executable | `genesis/a2o/features/**/*.feature` | story-coverage-audit.py |
| Narrative | `genesis/data/stories/*.md` | story-coverage-audit.py (consumer side) |

Today's audit measures features-vs-stories. It misses a strictly upstream failure mode:
specs that land, ship implementation, ship a feature file, ship the experience to users —
and *still never get a canonical story*. The protocol's promise (wisdom_resolves_into_epics)
requires that lived experience graduates into canonical narrative; if shipping outpaces
story-authoring, the gap is invisible and only surfaces years later as "we lost the why."

This is the **future-leaning instrument** the cartographer needs to see the storyteller's
horizon. The librarian sees present hygiene; the historian sees past precedent; the
storyteller sees disposition options. None of them currently see "what's shipped without a
story" — which is the cartographer's natural concern (the future-tense gap between what we
built and what we have made meaningful).

## What's blocking

- The story frontmatter schema for spec-references is informal — `sourced_from.spec_refs[]`
  is referenced in the storyteller-coverage-sprint backlog but is not yet a hard schema
  field. The audit can bootstrap with full-text match on spec slug until the schema lands,
  then tighten.
- Specs are heterogeneous (some are pure design, some are protocol-schema definitions,
  some are skill specs). The audit needs a heuristic for "spec deliverable shipped" — for
  now: spec has a referenced feature file with ≥1 passing scenario.

## What's ready

- story-coverage-audit.py is the canonical shape to mirror (`.claude/scripts/memory-kit/`)
- Spec directory has stable naming convention (YYYY-MM-DD-<slug>.md)
- Library helpers landing in `.claude/scripts/_lib/` (frontmatter parsing, slug-clean)
- Memkit Wave 1 prologue pattern is established and absorbs new audit scripts cleanly

## Convergence

- Cartographer Wave 1 (Run #3): future-surface flagged spec-without-story as the unmeasured
  axis adjacent to today's story-coverage-audit
- Storyteller Wave 2 (Run #3): noted that "sourced_from.spec_refs[]" wants to be a real
  schema field, which this audit motivates
- Storyteller-coverage-sprint backlog entry: declares the bootstrap pattern this audit
  would feed forward into

## Definition of done

1. Script exists at `.claude/scripts/memory-kit/spec-coverage-audit.py` matching the
   shape and idempotence guarantees of story-coverage-audit.py
2. Deterministic markdown output at `.claude/memory-kit/<date>/spec-coverage-audit.md`
   with: spec inventory, per-spec story-coverage classification (canonical / adjacent /
   none / not-yet-shipped), top-N "shipped without story" ranked by leverage
3. JSON sidecar at `.claude/memory-kit/spec-coverage-audit.json` (mirroring the existing
   audit's machine-readable companion)
4. Librarian's Wave 1 prologue invokes the script alongside story-coverage-audit
5. At least one subsequent ceremony's cartographer reads the output as Wave 1 substrate
   before this entry retires; if the data is consistently empty/uninformative, retire as
   "audit not load-bearing" rather than "audit completed"

## Sequencing

Lower priority than the storyteller-coverage-sprint and the
stewarded-device-sync-feature-authoring backlog entries — those have direct downstream
unblocking effects. This entry produces a future-tense instrument that becomes valuable
only after canonical stories exist at N≥3 and specs are routinely cross-referenced. Pick
up after the coverage sprint (Phase 1) lands and the storyteller has 2-3 canonical
stories at active.alpha or higher.

## Vision-alignment notes

- **Wisdom resolves into epics** (memory) — this audit measures the substrate that
  enables that resolution by surfacing where stories ought to exist but don't
- **Three temporal perspectives** (memory) — this is precisely the cartographer's
  future-tense instrument: which design-layer commitments have we made that the
  narrative layer has not yet caught up to
- **Forgetting as design** (memory) — by measuring "shipped without story," we make the
  forgetting visible at the substrate level rather than letting it accumulate silently

## Readiness score

**Vision-alignment 7/10** — strong principle alignment, but not foundational to a
ship-blocking deliverable. This is a substrate-instrument backlog entry, valuable but not
critical-path.

**Readiness 6/10** — schema dependency on `sourced_from.spec_refs[]` is soft; pattern
to mirror exists; deliverable is a single Python script; downstream consumers (librarian
Wave 1 + cartographer Wave 1) are ready to absorb it. Recommend authoring after at least
one more canonical story lands so the audit has meaningful test data.
