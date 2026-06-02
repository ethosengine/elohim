# Lamad Pillar — Learning & Content

The learning domain: the content graph, learning paths, mastery progression, knowledge
maps, and the assessment/renderer stack. Lamad is where a learner *journeys* — it turns
the protocol's substrate into a path someone can walk.

> **shaped by:** D2 (Evidence Primitives), D3 (Records Lifecycle & State Transitions),
> D9 (Economic Coordination & REA Interop). Seed epics that lamad realizes:
> `value_scanner/` (the care-economy REA primitive that makes invisible household work
> visible) and `social_medium/`. See the per-domain backlinks under
> [Architecture seeds that shape me](#architecture-seeds-that-shape-me).

---

## ⚠️ The SPA indirection — read this first

Lamad is **not** a directory under `app/elohim-app/src/app/` like the other app pillars.
It is a **separate Angular SPA** that lives at **`app/lamad/`**, with its code under
`app/lamad/src/app/`. The host app (`elohim-app`) reaches into it through a path alias:

```jsonc
// app/elohim-app/tsconfig.json
"@app/lamad/*": ["../lamad/src/app/*"]
```

So `import { ContentService } from '@app/lamad/...'` resolves into *this* SPA, not into
elohim-app. The host app lazy-loads a handful of lamad components directly
(`app.routes.ts` imports `@app/lamad/components/content-viewer/...`), and the full lamad
route tree (`lamad.routes.ts` here) is mounted under `/lamad`.

**Why two SPAs?** Lamad is the reference learning client — it can build and run
standalone (`cd app/lamad && pnpm start`) and is also composed into the federated
elohim-app surface. Treat it as its own buildable unit with its own `package.json`,
`vite.config.ts`, and test runner.

| Concern | Path |
|---------|------|
| Pillar code (this SPA) | `app/lamad/src/app/` |
| Path alias into it | `@app/lamad/*` → `app/lamad/src/app/*` (set in elohim-app tsconfig) |
| Domain vocabulary (manifest, schemas, coupling) | `elohim/sdk/domains/lamad/` |
| Generated types | `app/lamad/src/app/generated/` (run `pnpm run lamad:codegen`; never hand-edit) |
| Legacy reference notes | `app/lamad/docs/claude.md` (older partial guide; this file supersedes it) |

---

## Philosophy — Territory / Journey / Traveler / Maps

```
Territory (Content) → Journey (Paths) → Traveler (Progress) → Maps (Knowledge)
```

- **Territory** — `ContentNode`s are reusable, cross-pillar primitives (a concept, a
  scenario, a simulation). They are not owned by a single path.
- **Journey** — a `LearningPath` adds narrative order and context *over* territory.
  The same content can appear in many journeys.
- **Traveler** — mastery and progress are per-learner; they accrue against content, not
  against a path.
- **Maps** — four knowledge maps answer four questions (see below).

### Three load-bearing constraints

1. **Lazy loading** — never load "all paths" or "all content." Always fetch by id /
   reach / next-step.
2. **Fog of war** — a learner sees completed, current, or next step only.
3. **Territory vs Journey** — content is reusable; paths add the narrative. Don't fold
   path state into content state.

---

## Models

| Model | Purpose |
|-------|---------|
| `content-node.model.ts` | `ContentNode`, `ContentType`, `ContentReach`, `ContentRelationshipType`, `ContentMetadata` — the cross-pillar content primitive |
| `learning-path.model.ts` | `LearningPath`, `PathStep` — journey structure |
| `content-mastery.model.ts` | `ContentMastery` — Bloom's-taxonomy progression |
| `learner-mastery-profile.model.ts` | Aggregate mastery across content |
| `knowledge-map.model.ts` | Four map types (domain / self / person / collective) |
| `cluster-graph.model.ts` | Graph clustering for the explorer |
| `exploration.model.ts`, `exploration-context.model.ts` | Graph traversal queries |
| `search.model.ts` | `SearchQuery`, `SearchResult`, facets |
| `content-lifecycle.model.ts` | Active/Subordinate/Shelved/Closed (mirrors records-lifecycle, D3) |
| `content-attestation.model.ts`, `content-access.model.ts` | Reach + attestation gating (D2) |
| `path-extension.model.ts`, `path-negotiation.model.ts` | Learner path customization |
| `practice.model.ts`, `learning-points.model.ts` | Practice loops + points |
| `steward-economy.model.ts`, `stewardship-allocation.model.ts`, `trust-badge.model.ts` | REA / steward economics surfaced into the learning UI (D9) |
| `feedback-profile.model.ts`, `expertise-discovery.model.ts` | Reach-earning feedback + expertise signals |
| `human-node.model.ts`, `human-consent.model.ts`, `profile.model.ts` | Learner-side identity surfaced from imagodei |

## Services

Lamad has a large service layer. The load-bearing groups:

**Content & paths**
| Service | Purpose |
|---------|---------|
| `ContentService`, `content-backend.service.ts`, `content-resolver.service.ts` | Content access with reach checking + blob resolution |
| `PathService`, `path-context.service.ts`, `path-graph.service.ts` | Path & step navigation |
| `PathExtensionService`, `path-negotiation.service.ts`, `path-recommendation.service.ts`, `path-filter.service.ts` | Learner path customization & matching |
| `ContentMasteryService`, `mastery.service.ts`, `mastery-stats.service.ts` | Bloom's-taxonomy progression |
| `data-loader.service.ts` | Lazy, reach-aware data loading (re-exported from `@app/elohim`) |

**Knowledge & exploration**
| Service | Purpose |
|---------|---------|
| `KnowledgeMapService`, `hierarchical-graph.service.ts` | Four-dimensional knowledge maps |
| `ExplorationService`, `relationship.service.ts`, `related-concepts.service.ts` | Graph traversal, pathfinding |
| `SearchService` | Enhanced search with scoring |

**Blob / cache stack** (content-addressed delivery; mirrors the quilt/pantry vocabulary)
| Service | Purpose |
|---------|---------|
| `blob-manager.service.ts`, `blob-bootstrap.service.ts`, `blob-streaming.service.ts`, `blob-fallback.service.ts`, `blob-verification.service.ts`, `blob-cache-tiers.service.ts` | Blob fetch, verify (sha256), tiered cache, custodian fallback |
| `indexeddb-cache.service.ts`, `wasm-cache.service.ts`, `custodian-blob-distribution.service.ts` | Local cache + custodian distribution |

**Learner, economy & signals**
| Service | Purpose |
|---------|---------|
| `learner-context.service.ts`, `learner-backend-api.service.ts`, `assessment.service.ts`, `practice.service.ts`, `points.service.ts`, `exploration.service.ts` | Learner state, assessment sessions, practice loops |
| `steward-api.service.ts`, `stewardship-allocation.service.ts`, `trust-badge.service.ts`, `contributor-api.service.ts`, `projection-api.service.ts` | REA / steward economics + doorway projection (D9, D8) |
| `household-resilience.service.ts`, `resilience.service.ts` | Household/resilience surfacing (value_scanner seed) |
| `lamad-event.service.ts`, `signal-harness.service.ts` | Emits DNA signals as EPR-shaped events (D2: `dna-signal-as-epr-envelope`) |
| `progress-migration.service.ts` | Session → Holochain progress transfer (paired with imagodei migration) |
| `logger.service.ts` | Scoped logging |

## Guards

| Guard | Purpose |
|-------|---------|
| `lamadIdentityGuard` (`guards/lamad-identity.guard.ts`) | Requires network auth (hosted or steward). **Delegates to the `LAMAD_IDENTITY` token** (bound to `IdentityService` at the elohim-app composition root) so `lamad.routes.ts` carries **no direct imagodei import** — the cross-pillar boundary is held by a token, not a hard dependency. Most content routes are *not* guarded: access is gated by storage reach (commons/public = no auth). |

## Components

Components live under `components/<name>/`. The route-mounted set (`lamad.routes.ts`):

| Component | Route | Purpose |
|-----------|-------|---------|
| `LamadLayoutComponent` | (shell) | Layout wrapper for all lamad routes |
| `LamadHomeComponent` | `/lamad` | Path-centric landing / discovery |
| `PathOverviewComponent` | `/lamad/path/:pathId` | Path landing page |
| `PathNavigatorComponent` | `/lamad/path/:pathId/step/:stepIndex` | **The main learning UI** (step navigation) |
| `ContentViewerComponent` | `/resource/:resourceId` (app-level redirect) | Direct content viewing — lamad redirects to the app-level route since `ContentNode`s are cross-pillar |
| `ContentEditorPageComponent` | `/lamad/resource/:resourceId/edit` | Content editing (guarded) |
| `LearnerDashboardComponent` | `/lamad/me` | Learner dashboard |
| `ProfilePageComponent` | `/lamad/human` | Session-human profile management |
| `GraphExplorerComponent` | `/lamad/explore` | Visual knowledge map (Khan Academy style) |
| `MeaningMapComponent` | `/lamad/map` | List/card view alternative |
| `SearchComponent` | `/lamad/search` | Search interface |
| `LamadNotFoundComponent` | `/lamad/**` | Pillar-scoped 404 |

Supporting (non-route) components: `concept-card`, `mini-graph`, `path-navigator`,
`related-concepts-panel`, `affinity-circle`, `attention-flow`, `focused-view-toggle`.

## Subsystems (each has its own `claude.md` — read it before working there)

| Subsystem | Path | Purpose |
|-----------|------|---------|
| **Renderers** | `renderers/` | Content-format → component registry (`RendererRegistryService`, `RendererInitializerService`); `markdown-renderer`, `quiz-renderer`, `gherkin-renderer`, `iframe-renderer`. A format with no registered renderer falls through to the raw-JSON fallback — see CLAUDE.md "core vs extensible formats." |
| **Quiz engine** | `quiz-engine/` | The assessment instruments + scoring (`instruments/`, `services/`, `models/`). Wraps Sophia's Recognition callbacks; session/aggregation/interpretation live here, not in Sophia. |
| **Content I/O** | `content-io/` | Pluggable import/export plugins (`plugins/`, `interfaces/`, `content-io.module.ts`). |
| **Parsers** | `parsers/` | Content-body parsing (markdown, gherkin, etc.). |
| **Inspiration** | `inspiration/` | Discovery/serendipity surfacing. |

## Knowledge Map Types

| Type | Question | Inspiration |
|------|----------|-------------|
| Domain | What do I know? | Khan Academy |
| Self | Who am I? | "Know thyself" |
| Person | Who do I know? | Gottman Love Maps |
| Collective | What do we know? | Org knowledge mgmt |

## Content types & formats (use the lamad MANIFEST, not core protocol formats)

`ContentType` and `ContentFormat` are governed by the **lamad manifest**
(`elohim/sdk/domains/lamad/manifest.json`), codegen'd into
`generated/manifest-types.ts`. Seed data must use **lamad manifest formats**
(`sophia-quiz-json`, `html5-app`, `gherkin`, `markdown`), **not** broad core protocol
formats like `interactive` — the renderer registry only knows manifest formats, and a
core format falls through to the raw-JSON fallback. (This is the single most common
content-rendering bug; see root `CLAUDE.md` "core vs extensible formats.")

## Barrel re-exports — prefer direct imports

`models/index.ts` and `services/index.ts` re-export some cross-pillar symbols (e.g.,
`DataLoaderService`) for backward compatibility. Prefer the direct source:

```typescript
import { DataLoaderService } from '@app/elohim/services'; // preferred
import { DataLoaderService } from '@app/lamad/services';   // also works (re-export)
```

## Build & test (standalone SPA)

```bash
cd app/lamad
pnpm start                                   # ng serve (standalone)
pnpm run build                               # ng build
pnpm test                                    # vitest run --config vite.config.ts
pnpm exec vitest run --config vite.config.ts <pattern>   # single file
```

Regenerate types after a manifest/schema change:
```bash
pnpm run lamad:codegen        # → app/lamad/src/app/generated/manifest-types.ts
```

## Terminology

- "learner" / "journey" not "user" / "consumption"
- "territory" (content) vs "journey" (path) — keep them distinct
- "mastery" / "reach" / "steward" — these are protocol vocabulary, not UI labels

## Architecture seeds that shape me

This pillar's design is constrained by these canonical architecture seeds (the WALK from
manifesto → pillar → code; see `genesis/docs/content/elohim-protocol/architecture/`):

- **D2 — Evidence Primitives** —
  `architecture/2026-04-18-experience-story-epr-design.md`,
  `architecture/2026-05-11-attestation-consolidation-design.md`,
  `architecture/2026-05-15-dna-signal-as-epr-envelope.md`. Shapes content attestation,
  feedback profiles, and `lamad-event.service.ts` (signals as EPR envelopes).
- **D3 — Records Lifecycle & State Transitions** —
  `architecture/2026-05-24-records-lifecycle-design.md`. Shapes
  `content-lifecycle.model.ts` (Active/Subordinate/Shelved/Closed).
- **D9 — Economic Coordination & REA Interop** —
  `architecture/2026-05-20-wave3-valueflows-hrea-interop-design.md`. Shapes the steward /
  stewardship-allocation / trust-badge services and the `economic_coordination` seed epic.
- **Seed epics realized:** `genesis/docs/content/elohim-protocol/value_scanner/`
  (care-economy REA primitive → `household-resilience.service.ts`) and
  `genesis/docs/content/elohim-protocol/social_medium/`.

## See also

- Domain vocabulary: `elohim/sdk/domains/lamad/CLAUDE.md`
- Protocol schemas: `elohim/sdk/schemas/CLAUDE.md`
- Renderer details: `app/lamad/src/app/renderers/claude.md`
- Host-app deployment contexts & content-loading flow: `app/elohim-app/CLAUDE.md`
- Architecture graph (the GRAPH): `architecture/INDEX.md` — and the WALK: `architecture/MAP.md`
