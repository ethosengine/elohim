# Sprint 1B Design — Library B Pattern Stories for the Qahal Homepage

**Date:** 2026-05-22
**Sprint:** 1B (follows Sprint 1A, precedes Sprint 2)
**Author:** brainstorming session driven by the kickoff prompt at `genesis/docs/plans/2026-05-22-sprint-1b-library-b-kickoff-prompt.md`
**Companion roadmap entry:** `genesis/docs/plans/2026-05-21-qahal-mvp-roadmap.md` (between Sprint 1A and Sprint 2)

## 1 — Purpose

Sprint 1B authors the **Library B designed pattern stories** that compose Sprint 1A's 28 Lit elements (23 elohim-qahal + 5 elohim-imagodei) into the convergent Qahal homepage demonstration. Every story is a themed Storybook composition rendering one slice of the canonical narratives + the architecture vision against typed mock-data fixtures.

The sprint introduces **no new storage entities** and touches **no backend**. It is purely UI composition + Storybook authoring over the Library A elements that already exist.

The Storybook substrate (v10 + web-components-vite) is already wired and reachable; the `0.0.0.0` Che-endpoint binding originally listed in the kickoff prompt is **out of scope** for this sprint per operator direction.

## 2 — What this design hinges on

Three architectural decisions taken during brainstorming. Each was selected over named alternatives.

| Decision | Selected | Alternatives rejected | Why |
|---|---|---|---|
| **Composition strategy** | Shared composer module (`render-qahal-homepage.ts`) + thin per-scene story files | (a) Inline render per story — duplicates ~200 lines of chrome assembly × 18 files = 3,600 lines of boilerplate; (b) Single playground story driven by argTypes — destroys discoverability of canonical/variation/capability-gating artifacts | DRY + every story still gets a discoverable sidebar entry. One source of composition truth. |
| **Token-binding strategy** | Shared `_lib/qahal-decorator.ts` module exporting `qahalLightDecorator` / `qahalDarkDecorator` / `qahalHighContrastDecorator` | (a) Inline `EL_TOKENS` per story file — duplicates ~30 lines × 18 files = 540 lines; (b) Global `.storybook/preview.ts` decorator — touches the global Storybook surface, affects existing core stories | Scoped to qahal pillar, keeps "binding happens at decorator level" framing per `app/elohim-library/CLAUDE.md`, doesn't retrofit existing stories |
| **Variation breadth** | 7 variations across all 4 archetypes (3 household + 2 congregation + 1 life-group + 1 wisdom-commons) | (a) All 13 from UX Section 7 — ~50% longer sprint; (b) 1 per archetype — proves breadth but not edge-case depth | Demonstrates archetype + edge-case breadth without grinding through the full enumeration; remaining 6 deferred to a follow-on sprint |

## 3 — Architecture

Three layers of code, one direction of dependency:

```
.storybook/                  → designed/qahal/_lib/       → designed/qahal/homepage/__docs__/<bucket>/*.stories.ts
(unchanged)                    qahal-decorator.ts           (19 story files — 4 canonical + 7 variations + 2 toggle
                               render-qahal-homepage.ts        + 5 capability-gating + 1 playground;
                               story-controls.ts               ~30-40 lines each except playground at ~80)
                               types.ts
                                      ↑
                               default/qahal/fixtures/
                               canonical/*.ts (4 scene fixtures, narrative-faithful)
                               variations/*.ts (7 variation fixtures)
                               primitives/*.ts (4 existing mock-data modules — RELOCATED here)
```

### 3.1 The three new responsibilities

1. **Scene fixtures** — typed bundles at `default/qahal/fixtures/canonical/` and `variations/` that compose slices of the 4 existing primitives modules into named scenes. Each carries the rubric, the relevant member subset, the stream events, the social-compute topology, the co-steward observation, the curated EPRs, and the external links for that scene.

2. **`render-qahal-homepage.ts`** — a pure function `renderQahalHomepage(scene: Scene, opts: RenderOpts) → TemplateResult` that assembles the 4-column chrome (`<elohim-qahal-collective-switcher>` + `<elohim-qahal-sidebar>` + `<elohim-qahal-main-viewer>` + `<elohim-qahal-context-column>`) with the appropriate sidebar sections (protocol panels, curated EPRs, external links, power-user expandable), the active panel in the main viewer, and the persistent right-context column. Called by all 19 stories.

3. **`qahal-decorator.ts`** — exports `qahalLightDecorator`, `qahalDarkDecorator`, `qahalHighContrastDecorator`. Each wraps the story in a styled `<div>` containing the full `EL_TOKENS` block + every qahal-element's `--elohim-qahal-*-*`-to-`--el-*` token binding. The single source of brand-binding truth across all 19 homepage stories.

### 3.2 Constraints inherited from Library B discipline

Per `app/elohim-library/CLAUDE.md`:

- Stories NEVER modify the primitives' CSS, JSDoc, tag names, or behavior. Binding happens at the story-decorator level only.
- If a needed `@cssprop` doesn't exist on a primitive, file it as a `component-architect` follow-up. Don't reach inside the element.
- All mock data composes the three sources of truth: ts-rs generated views (where applicable), app-manifest vocabulary (rubric values, content formats), and graphos brand tokens (theme bindings).

### 3.3 Substrate that already exists

| Surface | Location | Sprint 1A status |
|---|---|---|
| Storybook v10 + web-components-vite | `app/elohim-library/.storybook/` | Wired; glob `../projects/graphos/**/__docs__/**/*.@(stories.ts\|mdx)` |
| 23 elohim-qahal Lit elements | `app/elohim-elements/elohim-qahal/src/` | All have capability profile JSDoc, three precondition gates, behavior + a11y tests, manifest-spec tests (433 passing) |
| 5 elohim-imagodei Lit elements | `app/elohim-elements/elohim-imagodei/src/` | Same discipline (100 passing) |
| 4 typed mock-data modules | `app/elohim-library/projects/graphos/src/default/qahal/fixtures/` | Will relocate to `fixtures/primitives/` in Sprint 1B |
| Brand-token convention | `hub-aggregation-shift.designed.stories.ts` | Pattern reference for decorator authoring |

## 4 — Directory layout

```
app/elohim-library/projects/graphos/src/
  designed/qahal/
    _lib/
      qahal-decorator.ts                # qahalLightDecorator / qahalDarkDecorator / qahalHighContrastDecorator
      qahal-decorator.spec.ts
      render-qahal-homepage.ts          # renderQahalHomepage(scene, opts) → TemplateResult
      render-qahal-homepage.spec.ts
      story-controls.ts                 # argTypes definitions for the playground story
      types.ts                          # Scene + RenderOpts type contracts

    homepage/__docs__/
      canonical/
        dowell-household.designed.stories.ts          # Designed/Qahal/Homepage/Canonical/Dowell Household
        cofc-congregation.designed.stories.ts         # Designed/Qahal/Homepage/Canonical/CofC Congregation
        hardins-life-group.designed.stories.ts        # Designed/Qahal/Homepage/Canonical/Hardins Life-Group
        wisdom-commons.designed.stories.ts            # Designed/Qahal/Homepage/Canonical/Wisdom Commons

      variations/
        household-with-toddlers.designed.stories.ts
        household-multi-generation.designed.stories.ts
        household-single-parent.designed.stories.ts
        congregation-doctrinal-tension.designed.stories.ts
        congregation-newly-formed.designed.stories.ts
        life-group-newly-formed.designed.stories.ts
        wisdom-commons-reconciliation-recorded.designed.stories.ts

      user-toggles/
        simple-user-view.designed.stories.ts          # Dowell, powerUserVisible=false
        power-user-view.designed.stories.ts           # Dowell, powerUserVisible=true

      capability-gating/
        visitor-view.designed.stories.ts              # Dowell, viewer = visitor
        engaged-view.designed.stories.ts              # Dowell, viewer = engaged
        contributor-view.designed.stories.ts          # Dowell, viewer = contributor
        steward-view.designed.stories.ts              # Dowell, viewer = steward
        protected-tier-view.designed.stories.ts       # Dowell, viewer = child (external-link section hidden)

      playground.designed.stories.ts                  # interactive controls over scene/tier/lens/power-user/panel

  default/qahal/fixtures/
    canonical/
      dowell-household-tuesday-morning.ts             # Scene fixture
      dowell-household-tuesday-morning.spec.ts        # narrative-fidelity tests
      cofc-congregation-sunday-morning.ts
      cofc-congregation-sunday-morning.spec.ts
      hardins-life-group-tuesday-evening.ts
      hardins-life-group-tuesday-evening.spec.ts
      wisdom-commons-thursday-afternoon.ts
      wisdom-commons-thursday-afternoon.spec.ts
    variations/
      household-with-toddlers.ts
      household-with-toddlers.spec.ts                 # typed-validity assertions
      household-multi-generation.ts
      household-multi-generation.spec.ts
      household-single-parent.ts
      household-single-parent.spec.ts
      congregation-doctrinal-tension.ts
      congregation-doctrinal-tension.spec.ts
      congregation-newly-formed.ts
      congregation-newly-formed.spec.ts
      life-group-newly-formed.ts
      life-group-newly-formed.spec.ts
      wisdom-commons-reconciliation-recorded.ts
      wisdom-commons-reconciliation-recorded.spec.ts
    primitives/                                       # the existing 4 mock-data modules — RELOCATED here
      mock-imagodei-profiles.ts
      mock-care-economy-events.ts
      mock-rubrics.ts
      mock-social-compute-topology.ts
```

### 4.1 Path migration

The 4 existing mock-data modules currently live directly under `default/qahal/fixtures/`. To make room for `canonical/` + `variations/` + `primitives/` substructure, they move into `fixtures/primitives/`. All import-paths in their consumers (existing tests + Library A default stories that reference them) update accordingly.

This is the only path migration in Sprint 1B.

### 4.2 Storybook sidebar shape after Sprint 1B

```
Designed/
  Core/
    elohim-compute-tile (already exists)
    ...
  Patterns/
    Hub-Aggregation-Shift (already exists)
  Qahal/
    Homepage/
      Canonical/
        Dowell Household
        CofC Congregation
        Hardins Life-Group
        Wisdom Commons
      Variations/
        Household With Toddlers
        Household Multi Generation
        Household Single Parent
        Congregation Doctrinal Tension
        Congregation Newly Formed
        Life Group Newly Formed
        Wisdom Commons Reconciliation Recorded
      User Toggles/
        Simple User View
        Power User View
      Capability Gating/
        Visitor View
        Engaged View
        Contributor View
        Steward View
        Protected Tier View
      Playground
```

## 5 — Type contracts

### 5.1 Scene fixture shape

```typescript
// designed/qahal/_lib/types.ts

import type {
  MockImagodeiProfile,
  CapabilityTier,
} from '../../../default/qahal/fixtures/primitives/mock-imagodei-profiles';
import type { MockReaEvent } from '../../../default/qahal/fixtures/primitives/mock-care-economy-events';
import type { MockRubric } from '../../../default/qahal/fixtures/primitives/mock-rubrics';
import type { MockSocialComputeTopology } from '../../../default/qahal/fixtures/primitives/mock-social-compute-topology';

/**
 * A Scene is a coherent slice of the Qahal substrate at one moment —
 * everything the homepage needs to render the storyteller's named moment.
 */
export interface Scene {
  id: string;
  qahalIcon: string;        // 🏠 ⛪ 🪨 🌳
  qahalLabel: string;
  qahalArchetype: 'household' | 'congregation' | 'life-group' | 'wisdom-commons';

  otherQahals: Array<{ id: string; icon: string; label: string }>;

  rubric: MockRubric;
  members: MockImagodeiProfile[];
  streamEvents: MockReaEvent[];
  computeTopology: MockSocialComputeTopology;
  coStewardObservation: string;

  curatedEprs: Array<{ id: string; title: string; provenance: 'curated-epr' }>;
  externalLinks: Array<{
    id: string;
    title: string;
    url: string;
    visibilityRequirement: CapabilityTier[];
  }>;
  pendingAcknowledgments: string[];   // event IDs in streamEvents
}
```

### 5.2 RenderOpts contract

```typescript
export interface RenderOpts {
  viewerTier: CapabilityTier;
  // 'visitor' | 'engaged' | 'contributor' | 'steward' | 'elohim-support'
  // | 'child' | 'idd_member' | 'elder_under_guardianship' | 'legal_steward_protected'

  powerUserVisible: boolean;   // imagodei-settings "Power-user view" preference

  lens: 'minimal' | 'simple' | 'standard' | 'detail' | 'debug' | 'trace';

  activePanel?: 'stream' | 'member-ring' | 'rules' | 'co-steward' | 'social-compute'
              | 'standing-inspector' | 'shefa-resources' | 'attestations' | 'graph-discovery';

  activeQahalId?: string;
  locale?: string;
  theme?: 'auto' | 'light' | 'dark';
}
```

### 5.3 Capability-gating discipline inside the composer

The composer reads `opts.viewerTier` against `scene.rubric.externalLinkVisibility` and renders external-link UI accordingly:

| Viewer tier | External link section behavior |
|---|---|
| `visitor` | Section filtered to co-steward-curated subset per rubric, OR hidden if rubric prescribes |
| `engaged` / `contributor` / `steward` | Section shown with all links |
| `child` | Section completely hidden (DOM-absent, not display:none) |
| `elder_under_guardianship` / `idd_member` | Filtered to co-steward-curated subset; rendered behind `<elohim-imagodei-protected-tier-marker>` |
| `legal_steward_protected` | Hidden, with `<elohim-imagodei-steward-configure-banner>` explanation |

The same DOM-absent discipline applies to `powerUserVisible=false`: when off, `<elohim-qahal-power-user-expandable>` is not in the rendered tree at all. The spec is strict that the toggle isn't a UX gesture — it's an imagodei preference, and its absence in the DOM reflects that.

## 6 — Story enumeration (19 files)

### 6.1 Canonical (4)

Each renders a storyteller-canonical moment from `genesis/docs/superpowers/specs/2026-05-21-qahal-section-4-canonical-narratives.md`. The Scene fixture must encode the named moments verbatim (sick James, Sheila's soup, Gertrude's check-in for Dowell; Brother Cal + Romans 12 + youth retreat for CofC; etc.).

Each canonical story renders at `viewerTier='steward'` + `lens='standard'` + `powerUserVisible=false` + `activePanel='stream'` — the default homepage view for the scene's primary protagonist.

### 6.2 Variations (7)

Each exercises a particular edge case the architecture must handle. Same composition function, different scene fixture. The 7 selected:

- `household-with-toddlers` — care-economy stream dominated by repetitive small care events; member-ring shows non-pilot tiers for the toddlers
- `household-multi-generation` — Gertrude-tier elder in the household; her capability tier influences external-link visibility per rubric
- `household-single-parent` — fewer adults, single steward, shows the floor of household participation
- `congregation-doctrinal-tension` — co-steward observation captures unresolved tension; sidebar surfaces dispute-mediation curated EPR
- `congregation-newly-formed` — sparse stream, no cohesion threshold reached yet, member-ring shows initial-tier composition
- `life-group-newly-formed` — counterpoint to the canonical Hardins (3 years cohesion); demonstrates the early-state surface
- `wisdom-commons-reconciliation-recorded` — counterpoint to the canonical Brother-Cal-concern-surface; shows the resolution moment

### 6.3 User toggles (2)

Both render the canonical Dowell household scene; the only difference is `powerUserVisible`:
- `simple-user-view` — `powerUserVisible=false`. Power-user-expandable section absent from DOM.
- `power-user-view` — `powerUserVisible=true`. Full sidebar including the 4 visual-stub panels.

### 6.4 Capability gating (5)

All render the canonical Dowell scene; the only difference is `viewerTier`:
- `visitor-view` — `viewerTier='visitor'`. External-link section filtered per rubric.
- `engaged-view` — `viewerTier='engaged'`. Full external-link visibility.
- `contributor-view` — `viewerTier='contributor'`. Same as engaged but additional power-user-eligible affordances.
- `steward-view` — `viewerTier='steward'`. Rules + co-steward panels editable.
- `protected-tier-view` — `viewerTier='child'` (James-as-child). External-link section DOM-absent; co-steward voice register softer; settings palette shows steward-configurable banner.

### 6.5 Playground (1)

Single interactive story with Storybook argTypes for `sceneId` (any canonical), `viewerTier`, `powerUserVisible`, `lens`, `activePanel`. Stable artifacts above remain authoritative; the playground complements with interactive exploration for stakeholder demos and the recognition+distinction Checkpoint F test.

```typescript
// designed/qahal/homepage/__docs__/playground.designed.stories.ts (sketch)

const SCENE_MAP = {
  'dowell-household-tuesday-morning': dowellTuesdayMorning,
  'cofc-congregation-sunday-morning': cofcSundayMorning,
  'hardins-life-group-tuesday-evening': hardinsTuesdayEvening,
  'wisdom-commons-thursday-afternoon': wisdomCommonsThursday,
};

const meta: Meta = {
  title: 'Designed/Qahal/Homepage/Playground',
  decorators: [qahalLightDecorator],
  argTypes: {
    sceneId: { control: 'select', options: Object.keys(SCENE_MAP) },
    viewerTier: { control: 'select', options: CAPABILITY_TIERS },
    powerUserVisible: { control: 'boolean' },
    lens: { control: 'select', options: LENSES },
    activePanel: { control: 'select', options: PANELS },
  },
  args: {
    sceneId: 'dowell-household-tuesday-morning',
    viewerTier: 'steward',
    powerUserVisible: false,
    lens: 'standard',
    activePanel: 'stream',
  },
};

export const Interactive: StoryObj = {
  render: (args) => renderQahalHomepage(SCENE_MAP[args.sceneId], { ...args, sceneId: undefined }),
};
```

## 7 — Test surface

Three layers, each answering a distinct correctness question.

### 7.1 Composer behavior tests (`_lib/render-qahal-homepage.spec.ts`)

Vitest. Asserts `renderQahalHomepage(scene, opts)` emits the right DOM structure for each opts combination. Verifies chrome assembly, capability gating, power-user gating, and panel routing — but not pixel-perfect rendering (that's Layer 3).

Coverage target: every opts combination producing a distinct DOM structure has a test. ~25 cases.

Cases include:
- Chrome assembly: all 4 columns present for unprotected tiers; lens + locale forwarded to every element
- External-link gating: 4 unprotected tiers see the section; `child` and `legal_steward_protected` see DOM-absent; `elder_under_guardianship` and `idd_member` see filtered + protected-tier-marker wrapper
- Power-user gating: `powerUserVisible=false` removes the expandable section; `=true` mounts the 4 power-user panels
- Panel routing: each `activePanel` value mounts the correct element in the main-viewer; context-column persists co-steward + condensed rules + condensed graph-discovery regardless

### 7.2 Scene-fixture narrative-fidelity tests (`fixtures/canonical/<name>.spec.ts`)

One spec per canonical scene. Asserts the fixture faithfully encodes the storyteller's named moments. These tests prevent canonical-narrative drift over sprint iterations.

**Dowell (Tuesday morning):** James present + tagged `child`; stream contains a `'sick'`-flagged James event; Sheila's `care`-kind `soup` event; Gertrude's `presence`-kind check-in; co-steward observation matches `/household.*steady/i`; 3 pending acknowledgments.

**CofC (Sunday morning):** Brother Cal present + tagged `steward`; 230 members; stream references Romans 12 sermon; youth retreat event needing 2 drivers; 3 prayer requests; co-steward observation references rising reach + 3 life-groups at cohesion threshold.

**Hardins (Tuesday evening):** 6 families gathered; Romans 12 v1 discussion event; Sarah's father hospital event; cohesion-threshold-reached signal; John Hardin hosting-accumulation event caught by friction-gradient.

**Wisdom Commons (Thursday afternoon):** 83 congregations in `otherQahals` reach; Brother Cal's concern surface event submitted to Arkansas sister congregation; peer council convening event; REA reconciliation event recorded.

**Variations** get lighter spec coverage — just typed-validity assertions (fixture conforms to `Scene` shape) since there's no canonical narrative to validate against.

### 7.3 Storybook test-runner (existing CI hook)

The repo already wires `pnpm test-storybook` via `@storybook/test-runner` mounting every story in headless Chrome with a11y check. Sprint 1B's 19 new story files participate automatically — no new CI surface, just more entries in the existing run.

### 7.4 Decorator tests (`_lib/qahal-decorator.spec.ts`)

~5 cases: each decorator function emits a wrapper div containing the expected `--el-*` brand tokens + per-element `--elohim-qahal-*-*` bindings; high-contrast decorator overrides border + state-color tokens per the design-spec contrast discipline.

## 8 — Verification ladder

Sprint 1B is done when, in order:

1. `pnpm test` passes for all new `*.spec.ts` (composer + decorator + scene fixtures) — ~50 new tests
2. `pnpm test-storybook` passes for all 19 new stories (no render errors, a11y clean)
3. `pnpm storybook` (local dev) shows the new sidebar structure: `Designed/Qahal/Homepage/{Canonical, Variations, User Toggles, Capability Gating, Playground}`
4. **Recognition test (Checkpoint F):** A non-technical observer opens `Designed/Qahal/Homepage/Canonical/Dowell Household` and recognizes the storyteller's Tuesday-morning scene — James + Sheila's soup + Gertrude's check-in + "household is steady"
5. **Distinction test (Checkpoint F):** The same observer, viewing `Designed/Qahal/Homepage/Capability Gating/Protected Tier View`, notices the external-link sidebar section is absent — the dignity-floor protection is visible

Stages 4-5 are operator-judgment, not automated. They're the gate from Sprint 1B to Sprint 2.

## 9 — What's in scope

- 19 Library B story files (4 canonical + 7 variations + 2 toggle + 5 capability-gating + 1 playground)
- `_lib/` module: `qahal-decorator.ts`, `render-qahal-homepage.ts`, `story-controls.ts`, `types.ts`, plus their specs
- 11 scene fixture files (4 canonical + 7 variations) plus 4 narrative-fidelity specs + 7 typed-validity specs
- Path migration of the 4 existing mock-data modules to `fixtures/primitives/` with import-path updates in their consumers
- A pillar-level README at `designed/qahal/README.md` explaining the composer + decorator pattern for future authors

## 10 — What's out of scope

- Storybook `0.0.0.0` Che endpoint binding (operator-directed cut from kickoff)
- CI hook for Storybook static-build PR review (deferred — operator may add later)
- Behavioral `.feature` scenarios for the homepage (deferred to Sprint 5 a2o authoring)
- Real backend wiring for any panel (Sprint 2+ substrate spine)
- The 6 remaining UX-spec variations (`household-with-teen`, `household-recovering-from-loss`, `congregation-at-peace`, `life-group-three-years-cohesive`, `life-group-departing-member`, `wisdom-commons-concern-surfaced`, `wisdom-commons-new-congregation-joining`) — deferred to a follow-on sprint that mines the value-scanner corpus
- New Lit primitives — Sprint 1B never reaches inside any element. If a needed `@cssprop` is missing, raise a `component-architect` follow-up, don't modify the element.
- Tier 3 substrate-extension primitives per the architecture vision's 18-item endgame

## 11 — Cross-references

- Sprint 1A plan: `genesis/docs/plans/2026-05-22-sprint-1a-elohim-elements-plan.md`
- Sprint 1B kickoff prompt: `genesis/docs/plans/2026-05-22-sprint-1b-library-b-kickoff-prompt.md`
- UX design spec (gospel for chrome + panel composition + capability gating): `genesis/docs/superpowers/specs/2026-05-22-qahal-homepage-ux-design.md`
- Architecture vision (Section 1.2 + 4 + 7.6a most relevant): `genesis/docs/superpowers/specs/2026-05-21-qahal-architecture-vision.md`
- Storyteller canonical narratives (the 5,067-word source for the named moments): `genesis/docs/superpowers/specs/2026-05-21-qahal-section-4-canonical-narratives.md`
- Library A / Library B authoring discipline: `app/elohim-library/CLAUDE.md`
- Conventions reference (decorator pattern source): `app/elohim-library/projects/graphos/src/designed/patterns/__docs__/hub-aggregation-shift.designed.stories.ts`
- Element conventions: `app/elohim-elements/elohim-qahal/CONVENTIONS.md`
- Companion roadmap: `genesis/docs/plans/2026-05-21-qahal-mvp-roadmap.md`
