---
id: lamad-mvp-implementation-arc
status: noted
tier: history
derived_from:
  - app/lamad/docs/IMPLEMENTATION_PLAN.md   # retired to git 2026-06-11 (lamad island recompose)
  - app/lamad/docs/IMPLEMENTATION_ARCHIVE.md
cites:
  - lamad-bundle-gospel | CLAUDE | sha256:e589f6c439666a7f | path: app/lamad/CLAUDE.md
  - lamad-domain-gospel | CLAUDE | sha256:0c7e351784c6df66 | path: elohim/sdk/domains/lamad/CLAUDE.md
  - app/lamad/src/app/services/data-loader.service.ts
  - app/lamad/src/app/services/practice.service.ts
  - subject-routing-locus-graph-design | 2026-06-11-subject-routing-locus-graph-design | sha256:a884cdf639a04699 | path: genesis/docs/superpowers/specs/2026-06-11-subject-routing-locus-graph-design.md
---

# The Lamad MVP Implementation Arc (Plan v5.0)

## What was built

`app/lamad/docs/IMPLEMENTATION_PLAN.md` ("Lamad MVP Implementation Plan v5.0", last
updated 2025-11-27) tracked 21 phases of the lamad learning platform MVP as a
phase-status table plus per-phase model summaries. The MVP was built as an Angular
*pillar inside elohim-app* against a JSON/localStorage prototype data layer
(plan §File Structure: `src/app/lamad/`, `src/assets/lamad-data/`). Twenty of
21 phases completed in that era — walking skeleton (paths, renderers, D3 graph
explorer), exploration/BFS, knowledge maps, bidirectional trust + badges, enhanced
search, session-human identity, Imago Dei profile, REA economic interface models,
four-dimensional relational maps, psychometric assessments, governance/deliberation
models, feedback-profile ("virality as a privilege"), and a cohesion review (ISO 8601
dates, attestation-model dedup). Only Phase 7 (Mastery) was deferred as "Post-MVP."

## What changed since (the plan never tracked it)

1. **The pillar dissolved into a bundle.** `app/elohim-app/src/app/` no longer
   contains a `lamad/` directory; lamad is now an independently-served EPR-app
   bundle at `app/lamad/` (`<base href="/lamad/">`, per `app/lamad/CLAUDE.md`).
   The plan's entire File Structure section describes a tree that no longer exists.
2. **"Post-MVP" phases shipped.** Phase 7 Mastery is live
   (`app/lamad/src/app/services/practice.service.ts:5` — "Orchestrates spaced
   repetition, discovery, and level progression"; `mastery.service.ts:115` freshness
   decay; design in `app/lamad/docs/BLOOM-MASTERY-DESIGN.md`). Holochain integration
   happened through exactly the adapter point the plan predicted: DataLoaderService
   now delegates to `ContentBackendService` (`data-loader.service.ts:49`) and marks
   the conductor "deprecated for content" (`data-loader.service.ts:298`) — reads flow
   through elohim-storage/doorway, and reach enforcement moved to the substrate
   (`elohim/epr/src/reach.rs`) rather than client-side checks.
3. **Cross-domain models migrated to their true homes.** Governance/deliberation
   models (Phases 18–19) now live in qahal
   (`app/elohim-app/src/app/qahal/models/governance-deliberation.model.ts`, with
   `proposal-vote` component); REA/economic models (Phase 15) dispersed into shefa
   (`app/elohim-app/src/app/shefa/models/exchange.model.ts`) and the ValueFlows
   bridge layer; session identity (Phase 13) moved to the `@elohim/identity` library
   (`app/lamad/src/app/services/session-human.service.spec.ts` imports
   `SessionHumanService` from `@elohim/identity`, "post M-AGGR-1 slimming").

## Why it matters for the future

- **The adapter-point bet paid off.** "DataLoaderService is the only service that
  knows about the data source" (plan §Holochain Migration Notes) was the single
  most durable architectural decision — the JSON→substrate migration happened
  through it without rewriting consumers.
- **Phase ≠ home.** Most v5.0 "phases" were model-authoring waves whose artifacts
  later belonged to other pillars (qahal, shefa, imagodei). A domain plan that
  authors cross-domain models accumulates a docs island; the subject-routing locus
  graph (`genesis/docs/superpowers/specs/2026-06-11-subject-routing-locus-graph-design.md`)
  is the corrective — lamad is a design-domain lens over the shared EPR core, and
  its docs decompose to protocol-canonical homes rather than restating substrate.
- **Status tables rot silently.** The table still reads "Phase 7: 🔮 Post-MVP" six
  months after mastery shipped, and lists priorities (Profile UI, trust badge UI,
  Cypress e2e) that landed long ago (`app/lamad/src/app/components/profile-page/`,
  trust badges in `content-viewer`, a2o coverage in `genesis/a2o/features/lms/`).
  Completion tracking belongs to living gates (CI, a2o, delivery scoreboard), not
  to a hand-updated markdown table.

The sibling `IMPLEMENTATION_ARCHIVE.md` (deprecated DocumentNode hierarchy,
graph-explorer drill-down panes) already preserved the era's deleted-code lessons;
this record closes the loop on the plan itself.
