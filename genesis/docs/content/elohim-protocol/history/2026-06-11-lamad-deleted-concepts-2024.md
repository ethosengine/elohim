---
id: lamad-deleted-concepts-2024
tier: history
layer: history record at the lamad reference-client locus (app/lamad). Assumes the lamad
  subject home (elohim/sdk/domains/lamad/ — lamad-domain-gospel) and the bundle gospel
  (app/lamad/CLAUDE.md — lamad-bundle-gospel); restates neither. Museum-shaped — lessons
  only, no live guidance.
derived_from:
  - app/lamad/docs/IMPLEMENTATION_ARCHIVE.md   # retired to git 2026-06-11 (lamad island recompose)
cites:
  - app/lamad/src/app/claude.md
  - app/lamad/src/app/lamad.routes.ts
  - lamad-domain-gospel | CLAUDE | sha256:0c7e351784c6df66 | path: elohim/sdk/domains/lamad/CLAUDE.md
  - subject-routing-locus-graph-design | 2026-06-11-subject-routing-locus-graph-design | sha256:a884cdf639a04699 | path: genesis/docs/superpowers/specs/2026-06-11-subject-routing-locus-graph-design.md
---

# Lamad Deleted Concepts (Nov 2024) — museum record

Distilled from `app/lamad/docs/IMPLEMENTATION_ARCHIVE.md` (Nov 2024, ~3,500 LOC removed:
14 model files, 6 component directories, 2 architectural patterns). The full inventory
lives in git history; this record keeps the lessons. Proximate retirement trigger: the
deprecation-sentinel false-fired on the word "Deprecated" inside this archive — an
un-routed island in the live scan path (locus-graph spec §7, lines 210-211).

## 1. DocumentNode inheritance hierarchy → flat ContentNode

A base-class hierarchy (`EpicNode`/`FeatureNode`/`ScenarioNode` extending `DocumentNode`,
15 files incl. adapter) was deleted in favor of a flat `ContentNode` with a `contentType`
string discriminator and a `metadata` bag (archive §"DocumentNode Abstraction", lines 11-44).

**Lessons:** composition over inheritance; Holochain entries are flat — inheritance doesn't
translate; YAGNI (epic→feature→scenario drilling was built before being needed); read the
spec twice before coding once; generate mock data matching interfaces before building services.

**Lineage:** the flat `contentType` discriminator is the seam the manifest-owned vocabulary
now occupies — 28 content-type declarations under
`elohim/sdk/domains/lamad/manifest/content-types/` and typed metadata via type guards
(`isPathNode()` etc., elohim/sdk/domains/lamad/CLAUDE.md lines 56-58, 99-109). The 2024
deletion is why a vocabulary file can add a content type without a class.

## 2. Graph-explorer drill-down components (6 directories)

Three-pane epic→feature→scenario drill-down UI, never routed in production, deleted
(archive lines 47-70). **Lessons:** route-first development — if it's not in
`lamad.routes.ts`, question whether it should exist; delete early, delete often; build for
the user flow (path navigation), not the data structure (graph). **Survivor:** a graph
explorer exists today as TERTIARY surface only (`/lamad/explore`,
app/lamad/src/app/lamad.routes.ts lines 21-23, 138-153) — the adaptation, not the deletion.

## 3. Standalone assessment model

`models/assessment.model.ts` deleted: never imported, premature abstraction before the
mastery system existed (archive lines 74-87). **Lessons:** wait for 2-3 concrete use cases
before a shared model; `grep` for importers — zero hits is the giveaway. **Lineage:**
assessment vocabulary is now subject-owned (`assessment.json`, `discovery-assessment.json`
content types; `sophia-quiz-json` format → sophia-renderer in
`elohim/sdk/domains/lamad/manifest/content-formats.json`).

## 4. Eager graph loading → lazy loading as a feature

`DocumentGraphService.buildGraph()` loaded ALL nodes on init — violated fog-of-war (locked
content inspectable in memory), didn't scale past ~100 nodes, wrong mental model for
Holochain (archive lines 93-110). **Lessons:** lazy loading enforces correct data-access
patterns, it is not an optimization; design data access as if the production backend already
exists. **Survivor:** live constraint at app/lamad/src/app/claude.md lines 62-64 and
src/app/services/claude.md line 21 ("NEVER create getAllPaths()").

## 5. Catch-all route pattern → explicit routes

`{ path: '**', component: GraphExplorerComponent }` handling hierarchical URLs was replaced
by explicit `path/:pathId/step/:stepIndex` + `resource/:resourceId` patterns (archive lines
113-134). **Lessons:** the route file should read like a URL spec; dynamic hierarchical
segments create coupling. **Survivor:** `'**'` today is only the lamad 404
(app/lamad/src/app/lamad.routes.ts lines 207-213).

## 6. Khan Academy "World of Math" inspiration (2012-2020)

The deprecation post-mortem Khan published shaped lamad's priority order: graph
visualization doesn't scale beyond ~200 nodes, users found it overwhelming, path-based
learning had better completion, mobile killed the big-canvas paradigm (archive lines
138-161). **Adaptation that survives:** path navigation PRIMARY, graph exploration
TERTIARY (lamad.routes.ts header comment, lines 7-28); four knowledge-map questions table
homed at app/lamad/src/app/claude.md lines 167-170.
