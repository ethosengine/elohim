---
id: "backlog-arch-frontend-bundle-seams-backlog"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Frontend bundle seams — pillar/core placement, the SDK↔bundle gospel chain, and the silent renderer-wiring gaps"
slug: "arch-frontend-bundle-seams-backlog"
written: "2026-08-11"
author: "claude (2026-08-11 substrate-currency ceremony — four-lens read + two adversarial verify lenses)"
status: "backlog"
priority: "medium"
tags: [architecture, frontend, pillars, lamad, angular, bundles, codegen, gospel-drift]
---

# Frontend bundle seams

Sibling of the `arch-*` dataplane clusters, on the other side of the boundary: how the Angular
bundles, the SDK domain homes, and the pillar table relate — and where that chain silently lies to
the next agent.

**Provenance:** all rows surfaced during the 2026-08-11 substrate-currency ceremony on
`elohim/sdk/domains/lamad/CLAUDE.md` and `.claude/skills/p2p-design-gate/SKILL.md` — a four-lens
read plus a diff-regression review and a fresh-context downstream-sprint simulation. Rows 1-2 are
the architectural decisions; rows 3-7 are gospel-chain drift; rows 8-10 are silent-failure gaps in
the codegen/wiring path. Evidence commands are inline per row and were run against the tree at
that date — re-run before acting, do not trust the counts.

## Rows

| # | Row | Shape | Notes |
|---|---|---|---|
| 1 | **lamad houses the cross-pillar content substrate inside the learning pillar.** `app/lamad/` holds both the learning domain (paths, mastery, quiz-engine) and the content substrate (`models/content-node.model`, `content-io/`, `renderers/`, `parsers/`). The substrate is imported by five pillars — elohim (15 files), avodah (5), shefa, qahal, imagodei — and `elohim/services/content.service.ts`, the cross-pillar core service, imports `ContentNode`/`ContentType`/`ContentReach` from `@app/lamad`. **The dependency arrow points core → pillar.** | Architect decision | Three candidate resolutions in "Row 1 detail" below. Blocks nothing, but every new pillar that renders content deepens it. `grep -rl "@app/lamad/models/content-node.model" app/elohim-app/src --include=*.ts` |
| 2 | **Two ContentServices, separate implementations.** `app/lamad/src/app/services/content.service.ts` (~831 lines, learning-flavored: `LAMAD_AGENT`, `DataLoaderService`, `LearningPath`) and `app/elohim-app/src/app/elohim/services/content.service.ts` (~948 lines, transport-flavored: `HttpClient`, `BLOB_FETCHER`, `ELOHIM_CLIENT`). Not a re-export. Neither file names the other. | Bounded task | Separable from row 1 and probably worth doing regardless: decide which owns transport and which owns learning semantics, and say so in both files. |
| 3 | ~~**Root `CLAUDE.md` Domain Pillars table places lamad under `app/elohim-app/src/app/`.**~~ **FIXED 2026-08-11.** Table now carries a per-pillar `Lives in` column, adds the missing `avodah` row, drops the non-resolving `import { ContentService } from '@app/lamad'` barrel example, and names the bundle-vs-domain seam + the core→pillar arrow inline. | Gospel fix | Was an always-in-context surface, so it mis-primed every session. |
| 4 | **`elohim/sdk/CLAUDE.md` points codegen at a nonexistent directory.** Claims both layers generate to `app/elohim-app/src/app/lamad/generated/`. Real targets per `codegen.mjs` `OUTPUT_DIRS`: `app/lamad/src/app/generated` + `genesis/seeder/src/generated`. | Gospel fix | |
| 5 | **`elohim/sdk/CLAUDE.md` contradicts itself on the ts-rs source crate.** Its prose (and root `CLAUDE.md`) say generated TS comes from `elohim/elohim-storage/src/views.rs`; its own SDK Boundary table says `elohim/elohim-views`. Ground truth: `views.rs` carries zero `#[derive(TS)]`; the derives live in `elohim/elohim-views/src/` (18 files). The table is right, the prose is stale — and the stale copy propagated into root gospel. | Gospel fix | Also: root `CLAUDE.md` and `elohim-storage/CLAUDE.md` disagree on the working directory for `cargo test export_bindings`. |
| 6 | **`elohim/sdk/CLAUDE.md`'s modular-manifest layout names files that do not exist** — `manifest/signal-kinds.json`, `manifest/projections.json`. Real concern files: `content-formats, relationships, signals, observations, observation-kinds, rendering, gates, attestations, graph` + `content-types/`. | Gospel fix | |
| 7 | **`elohim/sdk/schemas/CLAUDE.md` and `elohim/sdk/domains/CLAUDE.md` carry no `id:`/`cites:` frontmatter** — so the two most-cited-from SDK surfaces sit outside the content-addressed drift graph that is supposed to surface exactly rows 4-6. Both also carry live counts on a gospel surface (`[[feedback_agent_prompts_no_process_status]]`). | Gospel fix | Fixing this is what makes rows 4-6 self-surfacing next time instead of ceremony-discovered. |
| 8 | **A manifest-declared renderer that is not hand-wired fails silently.** `RENDERER_COMPONENTS` in `app/lamad/src/app/renderers/renderer-initializer.service.ts` is a hand-maintained name→class map; the initializer does `if (component) { register(...) }` with no `else`, no warn, no throw. `elohim-element-registry` is declared in `rendering.json` and absent from the map today — declared, never registered, no signal. | Bug | The failure presents as content falling to the raw-JSON fallback, i.e. it reads as a styling bug. |
| 9 | **Renderer/format collision is undetected.** `codegen.mjs` builds `rendererMap[fmt] = component` over `Object.entries` — last write wins, no duplicate-claim check, no schema constraint. Two renderers claiming one format silently resolves to whichever is enumerated last. | Bug | |
| 10 | **`spa-bundle` sits in `content-format.schema.json`'s flat `enum` but in neither `_tiers.core` nor `_tiers.extensible`.** Seed validation reads the flat enum so it validates fine; `_tiers` drives the generated core/extensible constants, so the format is invisible to codegen's tiering. A live format in a tier-less state. | Bug | Found while verifying a doc claim that assumed all domain formats live in `_tiers.extensible`. |

## Row 0 — why this class was invisible (added 2026-08-11, guard landed)

The rows above are symptoms of one root cause: **`app/lamad/` is a BUNDLE seam that got read as
a DOMAIN seam.** Extracting an EPR app buys an independently built, separately content-addressed
bundle — a deployment fact. It creates no module boundary, because each app's `tsconfig.json`
`paths` maps sibling `@app/<pillar>/*` aliases straight at the other workspace's private
`src/app/*`: deep paths, both directions, no public API. Two `package.json`s, one TS program.

Consequence: the directory *looks* like a boundary, so nothing checked that it was one. Each
individual cross-workspace import was locally reasonable. The arrow direction (core → pillar) and
the cycle were only visible in aggregate, and nothing aggregated them. Measured 2026-08-11:

| Direction | Specifiers | Refs | Heaviest edge |
|---|---|---|---|
| `elohim-app → lamad` | 38 | 129 | `models/content-node.model` (32) |
| `lamad → elohim-app` | 23 | 26 | spread across `models/ services/ interfaces/ utils/ guards/ quiz-engine/` |

`app/lamad/tsconfig.json`'s own comment asserted the reverse edges were composition-root-only.
They were not — 18 non-spec files crossed. **The confusion was written down and believed.** That
comment now points at the measured baseline instead of asserting cleanliness.

**Guard landed:** `app/scripts/lint-workspace-imports.mjs`, wired into both apps' `pnpm run lint`
alongside the existing `lint-route-literals` / `lint-ssr-entry` rails. It reads each app's own
tsconfig `paths`, keeps the `@app/*` aliases resolving outside the app root, counts every
reference, and ratchets against `app/scripts/workspace-import-baseline.json` (today's set as
*declared debt*, header naming what the debt is). Fails on a new specifier or a higher count;
shrinking reports and asks for a re-baseline; a cross-aliased app with no baseline entry fails
(an unmeasured seam is not a clean one). Both failure modes were probe-verified before wiring.

The rail takes no position on row 1 — it only guarantees the next drift of this class is a
decision somebody made rather than one that accumulated. Rows 1-2 remain open on the merits.

## Row 1 detail — candidate resolutions

Not yet chosen; needs an architect call. The ratchet holds the line meanwhile — it freezes the
entanglement, it does not resolve it.

1. **Promote the content substrate to a core home** (shared lib, or the elohim pillar), leaving
   `app/lamad/` with the learning domain only. Largest move; corrects the arrow for all five
   consumers.
2. **Declare `app/lamad/` the content-substrate home** and reframe the concern — "lamad" then names
   the content bundle rather than the learning pillar. Cheapest; requires the pillar table and the
   domain gospel to say so outright.
3. **Do row 2 only** (reconcile the ContentServices) and accept the placement as-is.

## Already done (documentation only — resolves nothing)

`elohim/sdk/domains/lamad/CLAUDE.md` now names the row-1 tension explicitly instead of implying
lamad is one clean domain with one client, and its Related Files table distinguishes the two
ContentServices. Root `CLAUDE.md`'s P2P-gate section was corrected in the same ceremony (a separate
concern — the stale entry-type headroom heuristic), but its **Domain Pillars table was not** — that
is row 3 and remains open.
