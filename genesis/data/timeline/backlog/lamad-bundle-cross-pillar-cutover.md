---
id: "backlog-lamad-bundle-cross-pillar-cutover"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Lamad bundle cross-pillar cutover — composition-root residue, retained tsconfig aliases, custom-element seam (B18c proper resolution)"
slug: "lamad-bundle-cross-pillar-cutover"
written: "2026-06-11"
author: "lamad island recompose (avodah authorship pass)"
status: "backlog"
priority: "medium"
tags: [lamad, bundle-independence, cross-pillar, composition-root, elohim-core, custom-elements, sdk-boundary]
cites:
  - genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md
  - genesis/docs/superpowers/plans/2026-05-25-pillar-epr-decomposition-plan.md
  - genesis/docs/superpowers/plans/2026-05-25-cross-pillar-import-cleanup.md
  - app/lamad/tsconfig.json
  - app/lamad/src/app/app.config.ts
  - app/lamad/src/app/components/content-viewer/content-viewer.component.ts
---

# Lamad bundle cross-pillar cutover — what remains of the B18c MVP shortcut

**Layer declaration:** parent design is the pillar-EPR decomposition spec
(`genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md`)
— lamad as an independently-served EPR-app bundle over the shared core. The
bundle rails it assumes are canonized in `app/lamad/CLAUDE.md`
(lamad-bundle-gospel); this entry tracks only the residual coupling work,
not the architecture.

## History (compressed from the app/lamad/docs/B18C-FOLLOWUP.md island)

Task B18 (decomposition plan, `2026-05-25-pillar-epr-decomposition-plan.md`
§Task B18) moved the lamad pillar out of elohim-app into `app/lamad/`. The
MVP shortcut: tsconfig path aliases pointing straight at elohim-app SOURCE,
so the bundle compiled but transitively bundled elohim-app code — breaking
changes in elohim-app break the lamad build, and the dependency direction
stayed coupled. The island listed ~25 symbols across EXTRACT /
STAY-CROSS-PILLAR / DUPLICATE / HTTP-API dispositions.

## What already resolved (verified 2026-06-11 — do not re-plan)

The successor plan
`genesis/docs/superpowers/plans/2026-05-25-cross-pillar-import-cleanup.md`
(159 imports, 8 disposition codes, 3 waves) absorbed the island's EXTRACT
bucket, and its Wave 2 substantially landed:

- The elohim-app lamad pillar is GONE (`app/elohim-app/src/app/lamad/` does
  not exist); lamad lives only in `app/lamad/`.
- Mastery types moved to the SDK: lamad imports `MasteryLevel`,
  `AgentProgress`, `getMasteryTier` from
  `@elohim/service/angular/models/agent.model`
  (`app/lamad/src/app/services/path.service.ts:9-14`).
- `AgentService` is decoupled via injection token: `LAMAD_AGENT` /
  `ILamadAgent` (`app/lamad/src/app/interfaces/agent.interface.ts`, header
  documents the Slice 2.1c P+inversion disposition).
- `DataLoaderService` is a local lamad service
  (`app/lamad/src/app/services/data-loader.service.ts`).
- New SDK libraries exist: `app/elohim-library/projects/elohim-identity/`
  and `elohim-rea-runtime/` (Slices 2.3/2.4 targets).
- COPY items done: `app/lamad/src/app/shared/services/seo.service.ts` exists.
- Lamad source is down to TWO cross-pillar-importing files (grep over
  `app/lamad/src`, excluding specs): `app.config.ts` (20 lines) and
  `content-viewer.component.ts` (1 line).

Also DEAD from the island:
- **Schema drift**: `content-icons.ts` carries `'element-registry'` (line 72)
  and `'element-registry-manifest'` (line 110) — the B18b fix held.
- **Codegen script TODO**: `codegen.mjs` still emits the relative
  `'../../generated/content-view'` import
  (`elohim/sdk/domains/lamad/scripts/codegen.mjs:521`), but it now RESOLVES
  inside the bundle — `app/lamad/src/generated/content-view.ts` exists, so
  from `src/app/generated/` the relative path lands correctly. The island's
  proposed fix (emit `@app/generated/content-view`) would today point at
  elohim-app via the alias and be a regression. Closed by structure.

## What remains (the actual backlog)

1. **Composition-root residue.** `app/lamad/src/app/app.config.ts` imports
   ~20 concrete elohim-app pillar services (`StorageApiService`,
   `AgentService`, `EprResolverService`, `GovernanceService`,
   `IdentityService` from imagodei, `EconomicEventsApiService` from shefa, …)
   for `LAMAD_*` token `useExisting` registration. The tsconfig comment
   (`app/lamad/tsconfig.json` paths block) is explicit: aliases are RETAINED
   under the Slice 2.1c composition-root pattern because these imports
   transitively reference `@app/{shefa,doorway,avodah,generated,testing}/*`.
   Consequence: the bundle still compiles elohim-app source transitively —
   the island's "bundle size includes elohim pillar services" and "breaking
   changes in elohim-app WILL break the lamad bundle build" impacts are
   STILL TRUE, just confined to one file.
2. **Custom-element seam (Slice 2.2b deferral).**
   `content-viewer.component.ts:103` imports `EprRelationshipsPanelComponent`
   from `@app/elohim` (documented as sanctioned composition-root import).
   The island's STAY-CROSS-PILLAR bucket mapped this family to future
   elements — `<elohim-epr-relationships-panel>`, `<elohim-epr-link>` etc.
   (decomposition plan Tasks B6/B8/B20; `<elohim-page-chrome>` and
   `<elohim-epr-link>` have since shipped per lamad-bundle-gospel rails —
   the relationships panel has not).
3. **Wave 3 cutover never ran.** The cleanup plan's cutover wave (remove
   cross-pillar aliases, verify standalone build, canonize the SDK boundary)
   is the definition-of-done for bundle independence; the plan's status is
   still `Draft` and the aliases are still in tsconfig.

Related (not duplicated here):
`genesis/data/timeline/backlog/epr-routing-complementary-captures.md`
proposes `provideLamadCrossPillarBridge()` to single-source the shell↔lamad
token bridge — that is the shell-side twin of residue item 1 and should be
designed together with it.

## Readiness

Concrete next slice: migrate the composition-root service imports'
TARGETS (the concrete services) into `@elohim/service`/SDK homes so
`app.config.ts` imports from libraries instead of elohim-app source, then
execute Wave 3 alias removal. Build verification is cheap and local
(`pnpm --filter lamad build` standalone).

OPEN QUESTION: several island HTTP-API dispositions (`EventService` → POST
/api/v1/economic-event, `AttentionTrackerService` → POST /api/v1/attention)
predate current substrate routes — whether those routes exist today was not
verified in this pass; re-audit against doorway routes before adopting that
disposition list.
