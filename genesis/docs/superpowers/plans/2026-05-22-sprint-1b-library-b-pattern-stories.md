# Sprint 1B — Library B Pattern Stories Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Author 19 Library B Storybook stories that compose Sprint 1A's 28 Lit elements into the convergent Qahal homepage, plus the shared composer + decorator + scene-fixture substrate that makes those stories thin and consistent.

**Architecture:** A shared `renderQahalHomepage(scene, opts)` function in `designed/qahal/_lib/` is called by every story file; a shared `qahalLightDecorator` / `qahalDarkDecorator` / `qahalHighContrastDecorator` module wraps every story in the brand-token canvas. Scene fixtures live at `default/qahal/fixtures/canonical/` + `variations/` and compose the existing primitives modules (which relocate to `fixtures/primitives/`).

**Tech Stack:** Lit 3 + TypeScript, Storybook v10 web-components-vite, Vitest + jsdom (already wired in `app/elohim-library/`), Elohim brand-token CSS custom properties (`--el-*`).

**Design spec:** `genesis/docs/superpowers/specs/2026-05-22-sprint-1b-library-b-design.md` (commit `795835b88`)

---

## File map

**Create (composer substrate, 6 files):**
- `app/elohim-library/projects/graphos/src/designed/qahal/_lib/types.ts`
- `app/elohim-library/projects/graphos/src/designed/qahal/_lib/qahal-decorator.ts`
- `app/elohim-library/projects/graphos/src/designed/qahal/_lib/qahal-decorator.spec.ts`
- `app/elohim-library/projects/graphos/src/designed/qahal/_lib/render-qahal-homepage.ts`
- `app/elohim-library/projects/graphos/src/designed/qahal/_lib/render-qahal-homepage.spec.ts`
- `app/elohim-library/projects/graphos/src/designed/qahal/README.md`

**Create (scene fixtures, 22 files: 11 fixtures + 11 specs):**
- `default/qahal/fixtures/canonical/dowell-household-tuesday-morning.ts` + `.spec.ts`
- `default/qahal/fixtures/canonical/cofc-congregation-sunday-morning.ts` + `.spec.ts`
- `default/qahal/fixtures/canonical/hardins-life-group-tuesday-evening.ts` + `.spec.ts`
- `default/qahal/fixtures/canonical/wisdom-commons-thursday-afternoon.ts` + `.spec.ts`
- `default/qahal/fixtures/variations/household-with-toddlers.ts` + `.spec.ts`
- `default/qahal/fixtures/variations/household-multi-generation.ts` + `.spec.ts`
- `default/qahal/fixtures/variations/household-single-parent.ts` + `.spec.ts`
- `default/qahal/fixtures/variations/congregation-doctrinal-tension.ts` + `.spec.ts`
- `default/qahal/fixtures/variations/congregation-newly-formed.ts` + `.spec.ts`
- `default/qahal/fixtures/variations/life-group-newly-formed.ts` + `.spec.ts`
- `default/qahal/fixtures/variations/wisdom-commons-reconciliation-recorded.ts` + `.spec.ts`

**Create (stories, 19 files):**
- `designed/qahal/homepage/__docs__/canonical/{dowell-household,cofc-congregation,hardins-life-group,wisdom-commons}.designed.stories.ts` (4)
- `designed/qahal/homepage/__docs__/variations/{household-with-toddlers,household-multi-generation,household-single-parent,congregation-doctrinal-tension,congregation-newly-formed,life-group-newly-formed,wisdom-commons-reconciliation-recorded}.designed.stories.ts` (7)
- `designed/qahal/homepage/__docs__/user-toggles/{simple-user-view,power-user-view}.designed.stories.ts` (2)
- `designed/qahal/homepage/__docs__/capability-gating/{visitor-view,engaged-view,contributor-view,steward-view,protected-tier-view}.designed.stories.ts` (5)
- `designed/qahal/homepage/__docs__/playground.designed.stories.ts` (1)

**Modify:**
- `app/elohim-library/vite.config.ts` — add `projects/graphos/src/**/*.spec.ts` to vitest `include`
- Move 4 existing fixture modules from `default/qahal/fixtures/` to `default/qahal/fixtures/primitives/`; update all import paths in consumers

**Path bases used in steps:** all paths are relative to `/projects/elohim/` unless otherwise noted.

---

### Task 1: Substrate prep — vitest pickup + fixture relocation

**Files:**
- Modify: `app/elohim-library/vite.config.ts`
- Move: `app/elohim-library/projects/graphos/src/default/qahal/fixtures/{mock-imagodei-profiles,mock-care-economy-events,mock-rubrics,mock-social-compute-topology}.ts` → `default/qahal/fixtures/primitives/`

- [ ] **Step 1.1: Add vitest include pattern for graphos specs**

In `app/elohim-library/vite.config.ts`, change the `include` array from:

```typescript
    include: [
      'src/**/*.spec.ts',
      'projects/elohim-service/src/resilience/**/*.spec.ts',
      'projects/elohim-service/src/distribution/**/*.spec.ts',
    ],
```

to:

```typescript
    include: [
      'src/**/*.spec.ts',
      'projects/elohim-service/src/resilience/**/*.spec.ts',
      'projects/elohim-service/src/distribution/**/*.spec.ts',
      'projects/graphos/src/**/*.spec.ts',
    ],
```

- [ ] **Step 1.2: Verify pickup with a single existing spec**

Run from `app/elohim-library/`:
```bash
pnpm test -- projects/graphos/src/default/qahal/fixtures/mock-imagodei-profiles.spec.ts 2>&1 | tail -20
```

Expected: either tests pass (if a spec exists for this module), OR "no test files found matching the include pattern" (proves the pattern is wired but no spec exists for this fixture). Either is a green signal.

- [ ] **Step 1.3: Enumerate fixture-module consumers before moving**

Run from repo root:
```bash
grep -rn "from.*['\"].*default/qahal/fixtures/mock-" \
  app/elohim-library/projects/graphos/src/ \
  app/elohim-elements/ \
  2>/dev/null
```

Expected: list of files that import the 4 fixture modules. Record this list — these are the files whose import paths must update in step 1.5.

- [ ] **Step 1.4: Create the `primitives/` subdir and move the 4 modules**

```bash
cd app/elohim-library/projects/graphos/src/default/qahal/fixtures
mkdir -p primitives
git mv mock-imagodei-profiles.ts primitives/
git mv mock-care-economy-events.ts primitives/
git mv mock-rubrics.ts primitives/
git mv mock-social-compute-topology.ts primitives/
ls -la
ls -la primitives/
```

Expected: `primitives/` directory contains the 4 `mock-*.ts` files; parent `fixtures/` directory contains only `primitives/`.

- [ ] **Step 1.5: Update import paths in all consumers**

For each file listed in step 1.3, replace import paths. The replacement pattern:
- `default/qahal/fixtures/mock-imagodei-profiles` → `default/qahal/fixtures/primitives/mock-imagodei-profiles`
- `default/qahal/fixtures/mock-care-economy-events` → `default/qahal/fixtures/primitives/mock-care-economy-events`
- `default/qahal/fixtures/mock-rubrics` → `default/qahal/fixtures/primitives/mock-rubrics`
- `default/qahal/fixtures/mock-social-compute-topology` → `default/qahal/fixtures/primitives/mock-social-compute-topology`

Also: the existing `mock-rubrics.ts` imports `./mock-imagodei-profiles.js` — that relative import inside `primitives/` stays the same (both files moved together).

After editing, run from `app/elohim-library/`:
```bash
pnpm test 2>&1 | tail -20
```

Expected: existing tests still pass — no broken imports.

- [ ] **Step 1.6: Commit**

```bash
git add app/elohim-library/vite.config.ts \
        app/elohim-library/projects/graphos/src/default/qahal/fixtures/primitives/ \
        $(grep -rln "default/qahal/fixtures/primitives/mock-" app/elohim-library/projects/graphos/src/ app/elohim-elements/ 2>/dev/null)
git commit -m "$(cat <<'EOF'
chore(library-b): relocate qahal fixture modules + extend vitest include

Move the 4 existing mock-data modules (imagodei profiles, care-economy
events, rubrics, social-compute topology) from default/qahal/fixtures/
to default/qahal/fixtures/primitives/ to make room for the canonical/
and variations/ scene-fixture substructure Sprint 1B introduces.

Extend vite.config.ts so vitest picks up specs under projects/graphos/.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: Type contracts (`_lib/types.ts`)

**Files:**
- Create: `app/elohim-library/projects/graphos/src/designed/qahal/_lib/types.ts`

- [ ] **Step 2.1: Author `types.ts`**

Create the file with:

```typescript
/**
 * Type contracts for the Qahal homepage Library B composer.
 *
 * - `Scene` is the fixture shape every canonical + variation file conforms to.
 *   It bundles a coherent slice of the Qahal substrate at one moment (the
 *   rubric, the relevant members, the stream, the social-compute topology,
 *   the co-steward observation, the curated EPRs + external links).
 *
 * - `RenderOpts` is the rendering-context payload every story passes to
 *   `renderQahalHomepage`. It drives capability gating, power-user gating,
 *   lens forwarding, and panel routing.
 *
 * - `QahalArchetype` discriminates the four worked-example archetypes; the
 *   composer reads it to vary panel set + sidebar shape per UX spec §3 + §4.
 */

import type {
  MockImagodeiProfile,
  CapabilityTier,
} from '../../../default/qahal/fixtures/primitives/mock-imagodei-profiles';
import type { MockCareEconomyEvent } from '../../../default/qahal/fixtures/primitives/mock-care-economy-events';
import type { MockRubric } from '../../../default/qahal/fixtures/primitives/mock-rubrics';
import type { MockComputeTopology } from '../../../default/qahal/fixtures/primitives/mock-social-compute-topology';

export type QahalArchetype = 'household' | 'congregation' | 'life-group' | 'wisdom-commons';

export type Lens = 'minimal' | 'simple' | 'standard' | 'detail' | 'debug' | 'trace';

export type ActivePanel =
  | 'stream'
  | 'member-ring'
  | 'rules'
  | 'co-steward'
  | 'social-compute'
  | 'standing-inspector'
  | 'shefa-resources'
  | 'attestations'
  | 'graph-discovery';

/** Reference to another Qahal the viewer participates in (collective-switcher icon row). */
export interface QahalReference {
  id: string;
  icon: string;
  label: string;
}

/** A curated EPR shown under the ◆ sidebar section. */
export interface CuratedEpr {
  id: string;
  title: string;
  provenance: 'curated-epr';
}

/** An external hyperlink shown under the ⤤ sidebar section (capability-gated by rubric). */
export interface ExternalLink {
  id: string;
  title: string;
  url: string;
  /** Capability tiers permitted to see this link per the household rubric. */
  visibilityRequirement: CapabilityTier[];
}

/**
 * A Scene — coherent slice of one Qahal at one moment.
 *
 * Each canonical scene is grounded in a specific narrative from
 * genesis/docs/superpowers/specs/2026-05-21-qahal-section-4-canonical-narratives.md.
 * Variations are synthetic but follow the same shape.
 */
export interface Scene {
  id: string;
  qahalIcon: string;
  qahalLabel: string;
  qahalArchetype: QahalArchetype;

  /** Other Qahals the viewer participates in — rendered as additional switcher icons. */
  otherQahals: QahalReference[];

  rubric: MockRubric;
  members: MockImagodeiProfile[];
  streamEvents: MockCareEconomyEvent[];
  computeTopology: MockComputeTopology;

  /** The co-steward's reflective observation (e.g., "the household is steady"). */
  coStewardObservation: string;

  curatedEprs: CuratedEpr[];
  externalLinks: ExternalLink[];

  /** Stream-event IDs flagged as awaiting acknowledgment (per UX §4.1). */
  pendingAcknowledgments: string[];
}

/**
 * Rendering context — what the story passes to `renderQahalHomepage`.
 */
export interface RenderOpts {
  /** Capability tier of the viewer. Drives external-link gating + protected-tier markers. */
  viewerTier: CapabilityTier;

  /** Imagodei-setting 'Power-user view' — true mounts power-user-expandable section. */
  powerUserVisible: boolean;

  /** Capability profile lens forwarded to every element. */
  lens: Lens;

  /** Active panel in the main viewer. Defaults to 'stream' inside the composer. */
  activePanel?: ActivePanel;

  /** Active Qahal id in the switcher. Defaults to scene.id. */
  activeQahalId?: string;

  /** Locale forwarded to every element. Defaults to 'en'. */
  locale?: string;

  /** Theme override. Defaults to 'auto' (decorator-level light/dark wins). */
  theme?: 'auto' | 'light' | 'dark';
}
```

- [ ] **Step 2.2: Type-check passes**

Run from `app/elohim-library/`:
```bash
pnpm exec tsc --noEmit -p tsconfig.json 2>&1 | tail -20
```

Expected: no type errors involving `types.ts`. (Some pre-existing errors in unrelated files are OK; verify none reference `_lib/types.ts`.)

- [ ] **Step 2.3: Commit**

```bash
git add app/elohim-library/projects/graphos/src/designed/qahal/_lib/types.ts
git commit -m "$(cat <<'EOF'
feat(library-b): add Scene + RenderOpts type contracts for qahal homepage

Centralizes the fixture + rendering-context types every Sprint 1B
homepage story and scene fixture conforms to.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: Shared decorator (`_lib/qahal-decorator.ts`)

**Files:**
- Create: `app/elohim-library/projects/graphos/src/designed/qahal/_lib/qahal-decorator.ts`
- Test: `app/elohim-library/projects/graphos/src/designed/qahal/_lib/qahal-decorator.spec.ts`

- [ ] **Step 3.1: Write the failing test first (TDD)**

Create `qahal-decorator.spec.ts`:

```typescript
import { describe, expect, it } from 'vitest';
import { html, render, type TemplateResult } from 'lit';
import {
  qahalLightDecorator,
  qahalDarkDecorator,
  qahalHighContrastDecorator,
} from './qahal-decorator';

function renderDecoratorToDom(decoratorFn: (story: () => TemplateResult) => TemplateResult) {
  const wrapper = decoratorFn(() => html`<elohim-qahal-test-child></elohim-qahal-test-child>`);
  const host = document.createElement('div');
  render(wrapper, host);
  return host;
}

describe('qahal-decorator', () => {
  describe('qahalLightDecorator', () => {
    it('emits a wrapper div with the EL_TOKENS brand block', () => {
      const host = renderDecoratorToDom(qahalLightDecorator);
      const wrapperStyle = host.querySelector('div')?.getAttribute('style') ?? '';
      expect(wrapperStyle).toContain('--el-cream');
      expect(wrapperStyle).toContain('--el-stone');
      expect(wrapperStyle).toContain('--el-green-deep');
    });

    it("renders the user's story content inside the wrapper", () => {
      const host = renderDecoratorToDom(qahalLightDecorator);
      expect(host.querySelector('elohim-qahal-test-child')).toBeTruthy();
    });

    it('sets the wrapper background to cream (light mode)', () => {
      const host = renderDecoratorToDom(qahalLightDecorator);
      const wrapperStyle = host.querySelector('div')?.getAttribute('style') ?? '';
      expect(wrapperStyle).toMatch(/background:\s*var\(--el-cream\)/);
    });
  });

  describe('qahalDarkDecorator', () => {
    it('sets the wrapper background to night (dark mode)', () => {
      const host = renderDecoratorToDom(qahalDarkDecorator);
      const wrapperStyle = host.querySelector('div')?.getAttribute('style') ?? '';
      expect(wrapperStyle).toMatch(/background:\s*var\(--el-night\)/);
    });
  });

  describe('qahalHighContrastDecorator', () => {
    it('emits explicit border + night-on-cream for max contrast', () => {
      const host = renderDecoratorToDom(qahalHighContrastDecorator);
      const wrapperStyle = host.querySelector('div')?.getAttribute('style') ?? '';
      expect(wrapperStyle).toContain('--el-night');
      expect(wrapperStyle).toMatch(/--elohim-qahal-.+-border:\s*2px solid/);
    });
  });
});
```

- [ ] **Step 3.2: Run the failing test**

```bash
cd app/elohim-library && pnpm test -- projects/graphos/src/designed/qahal/_lib/qahal-decorator.spec.ts 2>&1 | tail -20
```

Expected: FAIL with `Cannot find module './qahal-decorator'` or similar.

- [ ] **Step 3.3: Implement `qahal-decorator.ts`**

Create the file with:

```typescript
/**
 * Shared decorators for Library B Qahal-pillar pattern stories.
 *
 * Three exports — `qahalLightDecorator`, `qahalDarkDecorator`,
 * `qahalHighContrastDecorator` — each wraps a Storybook story in a styled
 * wrapper div carrying the full Elohim brand-token block (`--el-*`) plus
 * the per-element `--elohim-qahal-*-*` token bindings.
 *
 * The decorators are the single source of brand-binding truth across all
 * homepage stories. Per `app/elohim-library/CLAUDE.md`:
 *   - Stories NEVER modify primitives' CSS, JSDoc, tag names, or behavior
 *   - Binding happens at the story-decorator level only
 *
 * Brand-token reference: graphos/elohim-protocol-design-spec.md §14.
 * Pattern reference: hub-aggregation-shift.designed.stories.ts.
 */

import { html, type TemplateResult } from 'lit';

// ---------------------------------------------------------------------------
// Brand-token block — the full --el-* palette + spacing + typography stack
// ---------------------------------------------------------------------------

const EL_TOKENS = `
  --el-green-deep:  #2D5F3B;
  --el-green-light: #7FB069;
  --el-amber:       #D4A03E;
  --el-clay:        #B8664F;
  --el-cream:       #F5F0E8;
  --el-stone:       #6B6157;
  --el-sky:         #7BAFCB;
  --el-plum:        #6E4B6B;
  --el-starlight:   #E8E4D9;
  --el-night:       #0F1A12;
  --el-night-alt:   #1A1A2E;
  --el-font-display: 'Fraunces', Georgia, serif;
  --el-font-body:    'Source Serif 4', Georgia, serif;
  --el-font-ui:      'DM Sans', system-ui, sans-serif;
  --el-font-mono:    'JetBrains Mono', monospace;
  --el-space-xs:  8px;
  --el-space-sm:  16px;
  --el-space-md:  24px;
  --el-space-lg:  32px;
  --el-space-xl:  48px;
  --el-radius-sm: 4px;
  --el-radius-md: 8px;
  --el-radius-lg: 16px;
  --el-shadow-soft:   0 2px 8px rgba(107, 97, 87, 0.08);
  --el-shadow-medium: 0 4px 16px rgba(107, 97, 87, 0.12);
`;

// ---------------------------------------------------------------------------
// Per-element token bindings — light mode (the garden register)
// ---------------------------------------------------------------------------

const QAHAL_TOKENS_LIGHT = `
  /* chrome surface colors */
  --elohim-color-surface-0: var(--el-cream);
  --elohim-color-surface-1: var(--el-starlight);
  --elohim-color-surface-2: rgba(107, 97, 87, 0.08);
  --elohim-color-border:    rgba(107, 97, 87, 0.18);
  --elohim-color-focus:     var(--el-green-light);
  --elohim-color-accent:    var(--el-amber);
  --elohim-color-text:      var(--el-stone);
  --elohim-color-text-emphasis: var(--el-green-deep);

  /* qahal-pillar element token bindings */
  --elohim-qahal-collective-switcher-active-bg:  rgba(127, 176, 105, 0.18);
  --elohim-qahal-sidebar-bg:                     var(--el-cream);
  --elohim-qahal-sidebar-border:                 1px solid rgba(107, 97, 87, 0.18);
  --elohim-qahal-main-viewer-bg:                 var(--el-cream);
  --elohim-qahal-context-column-bg:              var(--el-starlight);
  --elohim-qahal-context-column-border:          1px solid rgba(107, 97, 87, 0.18);
  --elohim-qahal-stream-panel-divider:           1px solid rgba(107, 97, 87, 0.12);
  --elohim-qahal-stream-item-acknowledgment-border: 1px dashed var(--el-amber);
  --elohim-qahal-member-ring-fill:               var(--el-green-light);
  --elohim-qahal-member-ring-track:              var(--el-starlight);
  --elohim-qahal-standing-ring-fill:             var(--el-amber);
  --elohim-qahal-provenance-marker-curated-color: var(--el-green-deep);
  --elohim-qahal-provenance-marker-external-color: var(--el-stone);
  --elohim-qahal-capability-tier-chip-bg:        rgba(127, 176, 105, 0.12);
  --elohim-qahal-capability-tier-chip-fg:        var(--el-green-deep);
  --elohim-qahal-co-steward-panel-bg:            var(--el-starlight);
  --elohim-qahal-co-steward-panel-accent:        var(--el-plum);
`;

const QAHAL_TOKENS_DARK = `
  --elohim-color-surface-0: var(--el-night);
  --elohim-color-surface-1: var(--el-night-alt);
  --elohim-color-surface-2: rgba(232, 228, 217, 0.08);
  --elohim-color-border:    rgba(232, 228, 217, 0.18);
  --elohim-color-focus:     var(--el-amber);
  --elohim-color-accent:    var(--el-amber);
  --elohim-color-text:      var(--el-starlight);
  --elohim-color-text-emphasis: var(--el-amber);

  --elohim-qahal-collective-switcher-active-bg:  rgba(212, 160, 62, 0.18);
  --elohim-qahal-sidebar-bg:                     var(--el-night);
  --elohim-qahal-sidebar-border:                 1px solid rgba(232, 228, 217, 0.12);
  --elohim-qahal-main-viewer-bg:                 var(--el-night);
  --elohim-qahal-context-column-bg:              var(--el-night-alt);
  --elohim-qahal-context-column-border:          1px solid rgba(232, 228, 217, 0.12);
  --elohim-qahal-stream-panel-divider:           1px solid rgba(232, 228, 217, 0.08);
  --elohim-qahal-stream-item-acknowledgment-border: 1px dashed var(--el-amber);
  --elohim-qahal-member-ring-fill:               var(--el-green-light);
  --elohim-qahal-member-ring-track:              rgba(232, 228, 217, 0.12);
  --elohim-qahal-standing-ring-fill:             var(--el-amber);
  --elohim-qahal-provenance-marker-curated-color: var(--el-amber);
  --elohim-qahal-provenance-marker-external-color: var(--el-starlight);
  --elohim-qahal-capability-tier-chip-bg:        rgba(212, 160, 62, 0.18);
  --elohim-qahal-capability-tier-chip-fg:        var(--el-amber);
  --elohim-qahal-co-steward-panel-bg:            var(--el-night-alt);
  --elohim-qahal-co-steward-panel-accent:        var(--el-plum);
`;

const QAHAL_TOKENS_HIGH_CONTRAST = `
  --elohim-color-surface-0: var(--el-cream);
  --elohim-color-surface-1: var(--el-cream);
  --elohim-color-surface-2: rgba(15, 26, 18, 0.12);
  --elohim-color-border:    var(--el-night);
  --elohim-color-focus:     var(--el-night);
  --elohim-color-accent:    var(--el-green-deep);
  --elohim-color-text:      var(--el-night);
  --elohim-color-text-emphasis: var(--el-green-deep);

  --elohim-qahal-sidebar-bg:               var(--el-cream);
  --elohim-qahal-sidebar-border:           2px solid var(--el-night);
  --elohim-qahal-main-viewer-bg:           var(--el-cream);
  --elohim-qahal-context-column-bg:        var(--el-cream);
  --elohim-qahal-context-column-border:    2px solid var(--el-night);
  --elohim-qahal-stream-panel-divider:     1px solid var(--el-night);
`;

// ---------------------------------------------------------------------------
// Decorator factories
// ---------------------------------------------------------------------------

function buildWrapperStyle(themeTokens: string, background: string, color: string): string {
  return `
    ${EL_TOKENS}
    ${themeTokens}
    font-family: var(--el-font-ui);
    background: ${background};
    color: ${color};
    padding: var(--el-space-md);
    min-block-size: 100vh;
  `.replace(/\s+/g, ' ');
}

export function qahalLightDecorator(story: () => TemplateResult): TemplateResult {
  return html`
    <div style="${buildWrapperStyle(QAHAL_TOKENS_LIGHT, 'var(--el-cream)', 'var(--el-stone)')}">
      ${story()}
    </div>
  `;
}

export function qahalDarkDecorator(story: () => TemplateResult): TemplateResult {
  return html`
    <div style="${buildWrapperStyle(QAHAL_TOKENS_DARK, 'var(--el-night)', 'var(--el-starlight)')}">
      ${story()}
    </div>
  `;
}

export function qahalHighContrastDecorator(story: () => TemplateResult): TemplateResult {
  return html`
    <div style="${buildWrapperStyle(QAHAL_TOKENS_HIGH_CONTRAST, 'var(--el-cream)', 'var(--el-night)')}">
      ${story()}
    </div>
  `;
}
```

- [ ] **Step 3.4: Run tests to verify they pass**

```bash
cd app/elohim-library && pnpm test -- projects/graphos/src/designed/qahal/_lib/qahal-decorator.spec.ts 2>&1 | tail -20
```

Expected: PASS — 5 tests, all green.

- [ ] **Step 3.5: Commit**

```bash
git add app/elohim-library/projects/graphos/src/designed/qahal/_lib/qahal-decorator.ts \
        app/elohim-library/projects/graphos/src/designed/qahal/_lib/qahal-decorator.spec.ts
git commit -m "$(cat <<'EOF'
feat(library-b): add qahal-decorator module with light/dark/high-contrast

Centralizes the Elohim brand-token block (EL_TOKENS) and per-element
--elohim-qahal-*-* bindings into three reusable decorator factories.
Every Sprint 1B homepage story wraps its content via one of these
decorators rather than inlining the ~30-line token block per file.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 4: Composer chrome assembly (`_lib/render-qahal-homepage.ts`)

**Files:**
- Create: `app/elohim-library/projects/graphos/src/designed/qahal/_lib/render-qahal-homepage.ts`
- Test: `app/elohim-library/projects/graphos/src/designed/qahal/_lib/render-qahal-homepage.spec.ts`

This task builds the **chrome assembly** — emitting the four-column wrapper + slot wiring with no gating logic yet. Tasks 5–8 add gating + routing increments.

- [ ] **Step 4.1: Write the failing chrome-assembly tests**

Create `render-qahal-homepage.spec.ts`:

```typescript
import { describe, expect, it } from 'vitest';
import { render } from 'lit';
import { DOWELL_HOUSEHOLD_RUBRIC } from '../../../default/qahal/fixtures/primitives/mock-rubrics';
import { DOWELL_FAMILY } from '../../../default/qahal/fixtures/primitives/mock-imagodei-profiles';
import {
  DOWELL_TUESDAY_MORNING_STREAM,
} from '../../../default/qahal/fixtures/primitives/mock-care-economy-events';
import { DOWELL_HOUSEHOLD_TOPOLOGY } from '../../../default/qahal/fixtures/primitives/mock-social-compute-topology';
import type { Scene, RenderOpts } from './types';
import { renderQahalHomepage } from './render-qahal-homepage';

const baseScene: Scene = {
  id: 'dowell-household',
  qahalIcon: '🏠',
  qahalLabel: "Dowell Household",
  qahalArchetype: 'household',
  otherQahals: [
    { id: 'cofc-congregation', icon: '⛪', label: 'Local Churches of Christ' },
  ],
  rubric: DOWELL_HOUSEHOLD_RUBRIC,
  members: DOWELL_FAMILY,
  streamEvents: DOWELL_TUESDAY_MORNING_STREAM,
  computeTopology: DOWELL_HOUSEHOLD_TOPOLOGY,
  coStewardObservation: 'The household is steady.',
  curatedEprs: [
    { id: 'family-recipes', title: 'Family Recipes', provenance: 'curated-epr' },
  ],
  externalLinks: [
    {
      id: 'family-doc',
      title: 'Family Google Doc',
      url: 'https://docs.google.com/example',
      visibilityRequirement: ['engaged', 'contributor', 'steward'],
    },
  ],
  pendingAcknowledgments: [],
};

const baseOpts: RenderOpts = {
  viewerTier: 'steward',
  powerUserVisible: false,
  lens: 'standard',
};

function renderToHost(scene: Scene, opts: RenderOpts): HTMLElement {
  const host = document.createElement('div');
  render(renderQahalHomepage(scene, opts), host);
  return host;
}

describe('renderQahalHomepage — chrome assembly', () => {
  it('renders all four chrome elements', () => {
    const host = renderToHost(baseScene, baseOpts);
    expect(host.querySelector('elohim-qahal-collective-switcher')).toBeTruthy();
    expect(host.querySelector('elohim-qahal-sidebar')).toBeTruthy();
    expect(host.querySelector('elohim-qahal-main-viewer')).toBeTruthy();
    expect(host.querySelector('elohim-qahal-context-column')).toBeTruthy();
  });

  it('passes scene.qahalLabel to the sidebar as qahal-name', () => {
    const host = renderToHost(baseScene, baseOpts);
    const sidebar = host.querySelector('elohim-qahal-sidebar');
    expect(sidebar?.getAttribute('qahal-name')).toBe('Dowell Household');
  });

  it('passes the scene id as the active-collective-id of the switcher', () => {
    const host = renderToHost(baseScene, baseOpts);
    const switcher = host.querySelector('elohim-qahal-collective-switcher');
    expect(switcher?.getAttribute('active-collective-id')).toBe('dowell-household');
  });

  it('honors opts.activeQahalId override on the switcher', () => {
    const host = renderToHost(baseScene, { ...baseOpts, activeQahalId: 'cofc-congregation' });
    const switcher = host.querySelector('elohim-qahal-collective-switcher');
    expect(switcher?.getAttribute('active-collective-id')).toBe('cofc-congregation');
  });
});
```

- [ ] **Step 4.2: Run tests to verify they fail**

```bash
cd app/elohim-library && pnpm test -- projects/graphos/src/designed/qahal/_lib/render-qahal-homepage.spec.ts 2>&1 | tail -20
```

Expected: FAIL — `Cannot find module './render-qahal-homepage'`.

- [ ] **Step 4.3: Implement the chrome skeleton in `render-qahal-homepage.ts`**

Create the file with:

```typescript
/**
 * The single composer for all Library B Qahal-homepage stories.
 *
 * `renderQahalHomepage(scene, opts)` returns a Lit `TemplateResult` carrying
 * the four-column chrome assembly:
 *   1. <elohim-qahal-collective-switcher> — the far-left icon strip
 *   2. <elohim-qahal-sidebar> — the per-Qahal resource list
 *   3. <elohim-qahal-main-viewer> — the active panel
 *   4. <elohim-qahal-context-column> — persistent right-nav context
 *
 * Branching behavior:
 *   - External-link section gates on opts.viewerTier × scene.rubric.externalLinkVisibility
 *   - Power-user-expandable section gates on opts.powerUserVisible (DOM-absent when off)
 *   - Main-viewer mounts the element matching opts.activePanel (default 'stream')
 *   - Context column persists co-steward + condensed rules + condensed graph-discovery
 *
 * Per app/elohim-library/CLAUDE.md, the composer NEVER modifies any element's
 * internals — only assembles them with the public prop/slot surface.
 */

import { html, nothing, type TemplateResult } from 'lit';
import type { CapabilityTier } from '../../../default/qahal/fixtures/primitives/mock-imagodei-profiles';
import type { Scene, RenderOpts, ActivePanel } from './types';

// ---------------------------------------------------------------------------
// Element registration — load via barrel imports so Storybook + tests get all
// custom elements defined before the composer renders them.
// ---------------------------------------------------------------------------

import 'elohim-qahal/register';
import 'elohim-imagodei/register';

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

export function renderQahalHomepage(scene: Scene, opts: RenderOpts): TemplateResult {
  const activeQahalId = opts.activeQahalId ?? scene.id;
  const activePanel = opts.activePanel ?? 'stream';

  return html`
    <div class="qahal-homepage-chrome" style="display: grid; grid-template-columns: 80px 260px 1fr 280px; min-block-size: 100vh; gap: 0;">
      ${renderCollectiveSwitcher(scene, activeQahalId)}
      ${renderSidebar(scene, opts)}
      ${renderMainViewer(scene, opts, activePanel)}
      ${renderContextColumn(scene, opts)}
    </div>
  `;
}

// ---------------------------------------------------------------------------
// Column 1 — collective switcher
// ---------------------------------------------------------------------------

function renderCollectiveSwitcher(scene: Scene, activeQahalId: string): TemplateResult {
  const collectives = [
    { id: scene.id, icon: scene.qahalIcon, name: scene.qahalLabel },
    ...scene.otherQahals.map((q) => ({ id: q.id, icon: q.icon, name: q.label })),
  ];
  return html`
    <elohim-qahal-collective-switcher
      .collectives=${collectives}
      active-collective-id=${activeQahalId}
    ></elohim-qahal-collective-switcher>
  `;
}

// ---------------------------------------------------------------------------
// Column 2 — sidebar (resource list)
// ---------------------------------------------------------------------------

function renderSidebar(scene: Scene, opts: RenderOpts): TemplateResult {
  return html`
    <elohim-qahal-sidebar qahal-name=${scene.qahalLabel}>
      <elohim-qahal-protocol-panel-list slot="panels"></elohim-qahal-protocol-panel-list>
      <elohim-qahal-curated-epr-list
        slot="curated"
        .eprs=${scene.curatedEprs}
      ></elohim-qahal-curated-epr-list>
      ${renderExternalLinkSection(scene, opts)}
      ${renderPowerUserSection(scene, opts)}
    </elohim-qahal-sidebar>
  `;
}

function renderExternalLinkSection(scene: Scene, opts: RenderOpts): TemplateResult | typeof nothing {
  const visibility = scene.rubric.externalLinkVisibility[opts.viewerTier] ?? 'full';
  if (visibility === 'hidden') return nothing;
  const visibleLinks =
    visibility === 'filtered_via_co_steward'
      ? scene.externalLinks.filter((l) => l.visibilityRequirement.includes(opts.viewerTier))
      : scene.externalLinks;
  return html`
    <elohim-qahal-external-link-list
      slot="external"
      .links=${visibleLinks}
      filter-mode=${visibility}
    ></elohim-qahal-external-link-list>
  `;
}

function renderPowerUserSection(scene: Scene, opts: RenderOpts): TemplateResult | typeof nothing {
  if (!opts.powerUserVisible) return nothing;
  return html`
    <elohim-qahal-power-user-expandable slot="power-user"></elohim-qahal-power-user-expandable>
  `;
}

// ---------------------------------------------------------------------------
// Column 3 — main viewer (active panel)
// ---------------------------------------------------------------------------

function renderMainViewer(scene: Scene, opts: RenderOpts, activePanel: ActivePanel): TemplateResult {
  return html`
    <elohim-qahal-main-viewer active-panel-name=${activePanel}>
      ${renderActivePanel(scene, opts, activePanel)}
    </elohim-qahal-main-viewer>
  `;
}

function renderActivePanel(scene: Scene, opts: RenderOpts, activePanel: ActivePanel): TemplateResult {
  const streamEvents = scene.streamEvents.map((e) => ({
    id: e.id,
    authorId: e.authorId,
    timestamp: e.timestamp,
    content: e.content,
    rea: e.rea,
    acknowledgmentPending: scene.pendingAcknowledgments.includes(e.id),
  }));
  switch (activePanel) {
    case 'stream':
      return html`<elohim-qahal-stream-panel .events=${streamEvents}></elohim-qahal-stream-panel>`;
    case 'member-ring':
      return html`<elohim-qahal-member-ring-panel .members=${scene.members}></elohim-qahal-member-ring-panel>`;
    case 'rules':
      return html`<elohim-qahal-rules-panel .rubric=${scene.rubric}></elohim-qahal-rules-panel>`;
    case 'co-steward':
      return html`<elohim-qahal-co-steward-panel observation=${scene.coStewardObservation}></elohim-qahal-co-steward-panel>`;
    case 'social-compute':
      return html`<elohim-qahal-social-compute-panel .topology=${scene.computeTopology}></elohim-qahal-social-compute-panel>`;
    case 'standing-inspector':
      return html`<elohim-qahal-standing-inspector-panel></elohim-qahal-standing-inspector-panel>`;
    case 'shefa-resources':
      return html`<elohim-qahal-shefa-resources-panel></elohim-qahal-shefa-resources-panel>`;
    case 'attestations':
      return html`<elohim-qahal-attestations-panel></elohim-qahal-attestations-panel>`;
    case 'graph-discovery':
      return html`<elohim-qahal-graph-discovery-panel></elohim-qahal-graph-discovery-panel>`;
  }
}

// ---------------------------------------------------------------------------
// Column 4 — context column (persistent right-nav)
// ---------------------------------------------------------------------------

function renderContextColumn(scene: Scene, opts: RenderOpts): TemplateResult {
  return html`
    <elohim-qahal-context-column>
      <elohim-qahal-co-steward-panel
        slot="co-steward"
        observation=${scene.coStewardObservation}
        compact
      ></elohim-qahal-co-steward-panel>
      <elohim-qahal-rules-panel
        slot="rules"
        .rubric=${scene.rubric}
        compact
      ></elohim-qahal-rules-panel>
      <elohim-qahal-graph-discovery-panel
        slot="discovery"
        compact
      ></elohim-qahal-graph-discovery-panel>
    </elohim-qahal-context-column>
  `;
}

// Lens forwarding to every element is handled at the decorator level via
// CSS custom properties, not as individual props. If a future Sprint surfaces
// a lens-aware element prop, this composer will pass it through opts.lens.
export type { CapabilityTier };
```

- [ ] **Step 4.4: Run tests to verify chrome-assembly tests pass**

```bash
cd app/elohim-library && pnpm test -- projects/graphos/src/designed/qahal/_lib/render-qahal-homepage.spec.ts 2>&1 | tail -20
```

Expected: PASS — 4 chrome-assembly tests green.

- [ ] **Step 4.5: Commit**

```bash
git add app/elohim-library/projects/graphos/src/designed/qahal/_lib/render-qahal-homepage.ts \
        app/elohim-library/projects/graphos/src/designed/qahal/_lib/render-qahal-homepage.spec.ts
git commit -m "$(cat <<'EOF'
feat(library-b): add renderQahalHomepage composer with chrome assembly

Authors the shared composer all Sprint 1B homepage stories call. This
commit ships the chrome (collective-switcher + sidebar + main-viewer +
context-column) + sidebar resource sections + panel routing + base
capability gating for the external-link + power-user sections.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 5: Composer — external-link capability gating tests

**Files:**
- Test: `app/elohim-library/projects/graphos/src/designed/qahal/_lib/render-qahal-homepage.spec.ts`

The composer already implements gating in Task 4; this task adds explicit tests for each tier × visibility combination to prevent regression.

- [ ] **Step 5.1: Append capability-gating tests to the spec**

Append the following `describe` block to `render-qahal-homepage.spec.ts`:

```typescript
describe('renderQahalHomepage — external-link capability gating', () => {
  // Dowell rubric: visitor=filtered_via_co_steward; engaged/contributor/steward=full;
  // child/legal_steward_protected=hidden; idd_member/elder_under_guardianship=filtered_via_co_steward
  it.each(['engaged', 'contributor', 'steward'] as const)(
    'shows external-link section in full for viewerTier=%s',
    (tier) => {
      const host = renderToHost(baseScene, { ...baseOpts, viewerTier: tier });
      const list = host.querySelector('elohim-qahal-external-link-list');
      expect(list).toBeTruthy();
      expect(list?.getAttribute('filter-mode')).toBe('full');
    }
  );

  it('renders external-link section in filtered_via_co_steward mode for visitor', () => {
    const host = renderToHost(baseScene, { ...baseOpts, viewerTier: 'visitor' });
    const list = host.querySelector('elohim-qahal-external-link-list');
    expect(list?.getAttribute('filter-mode')).toBe('filtered_via_co_steward');
  });

  it.each(['child', 'legal_steward_protected'] as const)(
    'OMITS external-link section entirely from DOM for viewerTier=%s',
    (tier) => {
      const host = renderToHost(baseScene, { ...baseOpts, viewerTier: tier });
      expect(host.querySelector('elohim-qahal-external-link-list')).toBeNull();
    }
  );

  it.each(['idd_member', 'elder_under_guardianship'] as const)(
    'renders external-link section in filtered_via_co_steward mode for viewerTier=%s',
    (tier) => {
      const host = renderToHost(baseScene, { ...baseOpts, viewerTier: tier });
      const list = host.querySelector('elohim-qahal-external-link-list');
      expect(list?.getAttribute('filter-mode')).toBe('filtered_via_co_steward');
    }
  );
});

describe('renderQahalHomepage — power-user gating', () => {
  it('OMITS power-user-expandable when powerUserVisible=false', () => {
    const host = renderToHost(baseScene, { ...baseOpts, powerUserVisible: false });
    expect(host.querySelector('elohim-qahal-power-user-expandable')).toBeNull();
  });

  it('mounts power-user-expandable when powerUserVisible=true', () => {
    const host = renderToHost(baseScene, { ...baseOpts, powerUserVisible: true });
    expect(host.querySelector('elohim-qahal-power-user-expandable')).toBeTruthy();
  });
});
```

- [ ] **Step 5.2: Run tests — all should pass with the existing implementation**

```bash
cd app/elohim-library && pnpm test -- projects/graphos/src/designed/qahal/_lib/render-qahal-homepage.spec.ts 2>&1 | tail -25
```

Expected: PASS — all chrome + gating + power-user tests green (4 + ~9 + 2 = ~15 cases).

- [ ] **Step 5.3: Commit**

```bash
git add app/elohim-library/projects/graphos/src/designed/qahal/_lib/render-qahal-homepage.spec.ts
git commit -m "$(cat <<'EOF'
test(library-b): add capability-gating + power-user gating composer tests

Locks in the external-link visibility discipline (4 unprotected tiers
see full; child + legal_steward_protected are DOM-absent; visitor +
idd_member + elder_under_guardianship see filtered-via-co-steward) and
the power-user DOM-absent toggle behavior.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 6: Composer — panel routing + context-column persistence tests

**Files:**
- Test: `app/elohim-library/projects/graphos/src/designed/qahal/_lib/render-qahal-homepage.spec.ts`

- [ ] **Step 6.1: Append panel-routing tests**

Append to `render-qahal-homepage.spec.ts`:

```typescript
describe('renderQahalHomepage — panel routing', () => {
  const PANEL_TAG: Record<string, string> = {
    stream: 'elohim-qahal-stream-panel',
    'member-ring': 'elohim-qahal-member-ring-panel',
    rules: 'elohim-qahal-rules-panel',
    'co-steward': 'elohim-qahal-co-steward-panel',
    'social-compute': 'elohim-qahal-social-compute-panel',
    'standing-inspector': 'elohim-qahal-standing-inspector-panel',
    'shefa-resources': 'elohim-qahal-shefa-resources-panel',
    attestations: 'elohim-qahal-attestations-panel',
    'graph-discovery': 'elohim-qahal-graph-discovery-panel',
  };

  it.each(Object.entries(PANEL_TAG))(
    'mounts %s in the main-viewer when activePanel=%s',
    (panelName, expectedTag) => {
      const host = renderToHost(baseScene, { ...baseOpts, activePanel: panelName as never });
      const mainViewer = host.querySelector('elohim-qahal-main-viewer');
      expect(mainViewer?.querySelector(expectedTag)).toBeTruthy();
      expect(mainViewer?.getAttribute('active-panel-name')).toBe(panelName);
    }
  );

  it('defaults to stream panel when activePanel is not set', () => {
    const host = renderToHost(baseScene, baseOpts);
    const mainViewer = host.querySelector('elohim-qahal-main-viewer');
    expect(mainViewer?.querySelector('elohim-qahal-stream-panel')).toBeTruthy();
    expect(mainViewer?.getAttribute('active-panel-name')).toBe('stream');
  });

  it('forwards pendingAcknowledgments into stream events', () => {
    const scene = { ...baseScene, pendingAcknowledgments: [baseScene.streamEvents[0]!.id] };
    const host = renderToHost(scene, baseOpts);
    const streamPanel = host.querySelector('elohim-qahal-stream-panel') as HTMLElement & {
      events?: Array<{ id: string; acknowledgmentPending?: boolean }>;
    };
    expect(streamPanel?.events?.[0]?.acknowledgmentPending).toBe(true);
    expect(streamPanel?.events?.[1]?.acknowledgmentPending).toBeFalsy();
  });
});

describe('renderQahalHomepage — context-column persistence', () => {
  it('always renders co-steward + rules + discovery in context column regardless of activePanel', () => {
    for (const panel of ['stream', 'member-ring', 'rules', 'social-compute'] as const) {
      const host = renderToHost(baseScene, { ...baseOpts, activePanel: panel });
      const ctxCol = host.querySelector('elohim-qahal-context-column');
      expect(ctxCol?.querySelector('[slot="co-steward"]')).toBeTruthy();
      expect(ctxCol?.querySelector('[slot="rules"]')).toBeTruthy();
      expect(ctxCol?.querySelector('[slot="discovery"]')).toBeTruthy();
    }
  });

  it('renders context co-steward with scene.coStewardObservation', () => {
    const host = renderToHost(baseScene, baseOpts);
    const ctxCoSteward = host.querySelector('elohim-qahal-context-column [slot="co-steward"]');
    expect(ctxCoSteward?.getAttribute('observation')).toBe('The household is steady.');
  });
});
```

- [ ] **Step 6.2: Run tests**

```bash
cd app/elohim-library && pnpm test -- projects/graphos/src/designed/qahal/_lib/render-qahal-homepage.spec.ts 2>&1 | tail -30
```

Expected: PASS — all chrome + gating + power-user + panel-routing + context-column tests green (~25 cases total).

- [ ] **Step 6.3: Commit**

```bash
git add app/elohim-library/projects/graphos/src/designed/qahal/_lib/render-qahal-homepage.spec.ts
git commit -m "$(cat <<'EOF'
test(library-b): add panel-routing + context-column composer tests

Locks in the panel routing matrix (each of the 9 elohim-core panels
mounts in the main-viewer for its activePanel value; stream is default)
and the context-column persistence discipline (co-steward + rules +
graph-discovery always render in the right nav regardless of
main-viewer state).

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 7: Dowell household scene fixture + narrative-fidelity spec

**Files:**
- Create: `app/elohim-library/projects/graphos/src/default/qahal/fixtures/canonical/dowell-household-tuesday-morning.ts`
- Test: `.../canonical/dowell-household-tuesday-morning.spec.ts`

- [ ] **Step 7.1: Write the narrative-fidelity spec first (TDD)**

Create `dowell-household-tuesday-morning.spec.ts`:

```typescript
import { describe, expect, it } from 'vitest';
import { dowellHouseholdTuesdayMorning } from './dowell-household-tuesday-morning';

describe('dowell-household-tuesday-morning scene fixture', () => {
  const scene = dowellHouseholdTuesdayMorning;

  it('uses the Dowell household qahal id + label + archetype', () => {
    expect(scene.id).toBe('dowell-household');
    expect(scene.qahalLabel).toContain('Dowell');
    expect(scene.qahalArchetype).toBe('household');
    expect(scene.qahalIcon).toBe('🏠');
  });

  it('binds the DOWELL_HOUSEHOLD_RUBRIC', () => {
    expect(scene.rubric.qahalId).toBe('dowell-household');
    expect(scene.rubric.standingHonors).toContain('care contributed');
  });

  it('includes James as a child member', () => {
    const james = scene.members.find((m) => m.name.includes('James'));
    expect(james).toBeDefined();
    expect(james?.capabilityTier).toBe('child');
  });

  it('includes Gertrude as elder_under_guardianship', () => {
    const gertrude = scene.members.find((m) => m.name.includes('Gertrude'));
    expect(gertrude).toBeDefined();
    expect(gertrude?.capabilityTier).toBe('elder_under_guardianship');
  });

  it('encodes the sick-James moment in the stream', () => {
    const jamesEvent = scene.streamEvents.find((e) =>
      /james.*sick|sick.*james/i.test(e.content)
    );
    expect(jamesEvent).toBeDefined();
  });

  it("includes Sheila's soup event in the stream", () => {
    const soupEvent = scene.streamEvents.find(
      (e) => /sheila/i.test(e.content) && /soup/i.test(e.content)
    );
    expect(soupEvent).toBeDefined();
  });

  it("includes Gertrude's check-in event in the stream", () => {
    const checkIn = scene.streamEvents.find(
      (e) => /gertrude/i.test(e.content) && /check/i.test(e.content)
    );
    expect(checkIn).toBeDefined();
  });

  it("includes the co-steward observation 'the household is steady'", () => {
    expect(scene.coStewardObservation).toMatch(/household.*steady/i);
  });

  it('has at least 3 pending acknowledgments per the narrative', () => {
    expect(scene.pendingAcknowledgments.length).toBeGreaterThanOrEqual(3);
  });

  it('includes at least one curated EPR', () => {
    expect(scene.curatedEprs.length).toBeGreaterThan(0);
  });

  it('includes the cofc-congregation as an otherQahal', () => {
    expect(scene.otherQahals.some((q) => q.id === 'cofc-congregation')).toBe(true);
  });
});
```

- [ ] **Step 7.2: Run test — should fail (module not yet exists)**

```bash
cd app/elohim-library && pnpm test -- projects/graphos/src/default/qahal/fixtures/canonical/dowell-household-tuesday-morning.spec.ts 2>&1 | tail -10
```

Expected: FAIL — `Cannot find module './dowell-household-tuesday-morning'`.

- [ ] **Step 7.3: Author `dowell-household-tuesday-morning.ts`**

Create the file with:

```typescript
/**
 * Canonical scene fixture — Dowell Household, Tuesday morning.
 *
 * Grounded in storyteller canonical narrative §4.1 at
 * genesis/docs/superpowers/specs/2026-05-21-qahal-section-4-canonical-narratives.md.
 *
 * Named moments encoded:
 *   - James (11) is sick (sore throat, hard week at school)
 *   - Sheila (Matthew's sister) sent chicken-and-rice soup the night before
 *   - Gertrude (Matthew's mother, half a continent away) checked in
 *   - 3 pending acknowledgments (Sheila's soup, Gertrude's check-in, neighbor's offer to bring dinner)
 *   - Co-steward observation: "the household is steady"
 *
 * This fixture composes the existing primitives modules without authoring
 * new mock data from scratch — the DOWELL_FAMILY, DOWELL_HOUSEHOLD_RUBRIC,
 * DOWELL_TUESDAY_MORNING_STREAM, and DOWELL_HOUSEHOLD_TOPOLOGY are
 * authoritative; this file binds them into a coherent Scene.
 */

import type { Scene } from '../../../../designed/qahal/_lib/types';
import { DOWELL_FAMILY } from '../primitives/mock-imagodei-profiles';
import { DOWELL_HOUSEHOLD_RUBRIC } from '../primitives/mock-rubrics';
import { DOWELL_TUESDAY_MORNING_STREAM } from '../primitives/mock-care-economy-events';
import { DOWELL_HOUSEHOLD_TOPOLOGY } from '../primitives/mock-social-compute-topology';

// Pending acknowledgments — IDs from DOWELL_TUESDAY_MORNING_STREAM.
// Resolve at fixture-load time by matching content against the narrative's
// three pending items: Sheila's soup, Gertrude's check-in, the neighbor's dinner offer.
const pendingAcknowledgmentIds = DOWELL_TUESDAY_MORNING_STREAM.filter((event) => {
  const c = event.content.toLowerCase();
  return (
    (c.includes('sheila') && c.includes('soup')) ||
    (c.includes('gertrude') && c.includes('check')) ||
    (c.includes('neighbor') && (c.includes('dinner') || c.includes('bring')))
  );
}).map((event) => event.id);

export const dowellHouseholdTuesdayMorning: Scene = {
  id: 'dowell-household',
  qahalIcon: '🏠',
  qahalLabel: 'Dowell Household',
  qahalArchetype: 'household',
  otherQahals: [
    { id: 'cofc-congregation', icon: '⛪', label: 'Local Churches of Christ' },
    { id: 'hardins-life-group', icon: '🪨', label: 'Tuesday Life-Group' },
    { id: 'wisdom-commons', icon: '🌳', label: 'Wisdom Commons' },
  ],
  rubric: DOWELL_HOUSEHOLD_RUBRIC,
  members: DOWELL_FAMILY,
  streamEvents: DOWELL_TUESDAY_MORNING_STREAM,
  computeTopology: DOWELL_HOUSEHOLD_TOPOLOGY,
  coStewardObservation: 'The household is steady.',
  curatedEprs: [
    { id: 'family-recipes', title: 'Family Recipes', provenance: 'curated-epr' },
    { id: 'birthday-calendar', title: 'Birthday Calendar', provenance: 'curated-epr' },
    { id: 'sick-day-playlist', title: 'Sick-Day Playlist', provenance: 'curated-epr' },
  ],
  externalLinks: [
    {
      id: 'family-google-doc',
      title: 'Family Google Doc',
      url: 'https://docs.google.com/example',
      visibilityRequirement: ['engaged', 'contributor', 'steward'],
    },
  ],
  pendingAcknowledgments: pendingAcknowledgmentIds,
};
```

- [ ] **Step 7.4: Run tests — should pass**

```bash
cd app/elohim-library && pnpm test -- projects/graphos/src/default/qahal/fixtures/canonical/dowell-household-tuesday-morning.spec.ts 2>&1 | tail -20
```

Expected: PASS — all 11 narrative-fidelity tests green.

If any test fails because the existing `DOWELL_TUESDAY_MORNING_STREAM` doesn't contain matching content for a narrative beat, treat that as a fixture-content gap: open `primitives/mock-care-economy-events.ts`, add the missing event to the relevant scene array (preserving the canonical narrative phrasing), commit that fix separately first, then re-run.

- [ ] **Step 7.5: Commit**

```bash
git add app/elohim-library/projects/graphos/src/default/qahal/fixtures/canonical/dowell-household-tuesday-morning.ts \
        app/elohim-library/projects/graphos/src/default/qahal/fixtures/canonical/dowell-household-tuesday-morning.spec.ts
git commit -m "$(cat <<'EOF'
feat(library-b): add Dowell household Tuesday-morning scene fixture

Composes DOWELL_FAMILY + DOWELL_HOUSEHOLD_RUBRIC +
DOWELL_TUESDAY_MORNING_STREAM + DOWELL_HOUSEHOLD_TOPOLOGY into a Scene
that renders the storyteller's canonical §4.1 moment. Narrative
fidelity locked by 11 tests covering the sick-James moment, Sheila's
soup, Gertrude's check-in, the 3 pending acknowledgments, and the
"household is steady" co-steward observation.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 8: CofC congregation Sunday-morning scene fixture + spec

**Files:**
- Create: `.../canonical/cofc-congregation-sunday-morning.ts` + `.spec.ts`

- [ ] **Step 8.1: Write the narrative-fidelity spec**

Create `cofc-congregation-sunday-morning.spec.ts`:

```typescript
import { describe, expect, it } from 'vitest';
import { cofcCongregationSundayMorning } from './cofc-congregation-sunday-morning';

describe('cofc-congregation-sunday-morning scene fixture', () => {
  const scene = cofcCongregationSundayMorning;

  it('uses the CofC congregation qahal id + congregation archetype', () => {
    expect(scene.id).toBe('cofc-congregation');
    expect(scene.qahalArchetype).toBe('congregation');
    expect(scene.qahalIcon).toBe('⛪');
  });

  it('binds the COFC_CONGREGATION_RUBRIC with 4 elder stewards', () => {
    expect(scene.rubric.qahalId).toBe('cofc-congregation');
    expect(scene.rubric.configuredBy).toHaveLength(4);
    expect(scene.rubric.configuredBy).toContain('brother-cal');
  });

  it('includes Brother Cal as a steward', () => {
    const cal = scene.members.find((m) => m.id === 'brother-cal');
    expect(cal).toBeDefined();
    expect(cal?.standingTier).toBe('steward');
  });

  it('references the Romans 12 sermon series in the stream', () => {
    expect(scene.streamEvents.some((e) => /romans 12/i.test(e.content))).toBe(true);
  });

  it('mentions the youth retreat needing drivers', () => {
    expect(
      scene.streamEvents.some((e) => /youth.*retreat/i.test(e.content) || /drivers?/i.test(e.content))
    ).toBe(true);
  });

  it('co-steward observation references rising reach OR life-groups at threshold', () => {
    expect(scene.coStewardObservation).toMatch(/(reach.*rising|life.?groups?.*threshold|cohesion)/i);
  });
});
```

- [ ] **Step 8.2: Run — should fail**

```bash
cd app/elohim-library && pnpm test -- projects/graphos/src/default/qahal/fixtures/canonical/cofc-congregation-sunday-morning.spec.ts 2>&1 | tail -10
```

Expected: FAIL — module missing.

- [ ] **Step 8.3: Author the fixture**

Create `cofc-congregation-sunday-morning.ts`:

```typescript
/**
 * Canonical scene fixture — CofC Congregation, Sunday morning.
 *
 * Grounded in storyteller canonical narrative §4.2.
 *
 * Named moments encoded:
 *   - 230 members; 4 elders (Brother Cal, Thompson, Davis, Rhodes)
 *   - Romans 12 sermon series, fourth week
 *   - Youth retreat needing 2 drivers
 *   - 3 prayer requests
 *   - Co-steward observation: reach into the neighborhood is rising; 3 life-groups at cohesion threshold
 */

import type { Scene } from '../../../../designed/qahal/_lib/types';
import { COFC_ELDERS } from '../primitives/mock-imagodei-profiles';
import { COFC_CONGREGATION_RUBRIC } from '../primitives/mock-rubrics';
import { COFC_SUNDAY_MORNING_STREAM } from '../primitives/mock-care-economy-events';
import { COFC_CONGREGATION_TOPOLOGY } from '../primitives/mock-social-compute-topology';

export const cofcCongregationSundayMorning: Scene = {
  id: 'cofc-congregation',
  qahalIcon: '⛪',
  qahalLabel: 'Local Churches of Christ',
  qahalArchetype: 'congregation',
  otherQahals: [
    { id: 'dowell-household', icon: '🏠', label: 'Dowell Household' },
    { id: 'hardins-life-group', icon: '🪨', label: 'Tuesday Life-Group' },
    { id: 'wisdom-commons', icon: '🌳', label: 'Wisdom Commons' },
  ],
  rubric: COFC_CONGREGATION_RUBRIC,
  members: COFC_ELDERS,
  streamEvents: COFC_SUNDAY_MORNING_STREAM,
  computeTopology: COFC_CONGREGATION_TOPOLOGY,
  coStewardObservation:
    "The congregation's reach into the neighborhood is rising slightly. Three life-groups are nearing the cohesion threshold.",
  curatedEprs: [
    { id: 'romans-12-series', title: 'Romans 12 Sermon Series', provenance: 'curated-epr' },
    { id: 'communion-rota', title: 'Communion Rota', provenance: 'curated-epr' },
    { id: 'youth-retreat-plans', title: 'Youth Retreat Plans', provenance: 'curated-epr' },
  ],
  externalLinks: [
    {
      id: 'congregation-website',
      title: 'Congregation Website',
      url: 'https://example.church',
      visibilityRequirement: ['engaged', 'contributor', 'steward'],
    },
  ],
  pendingAcknowledgments: COFC_SUNDAY_MORNING_STREAM.filter((e) =>
    /retreat.*driver|prayer request/i.test(e.content)
  )
    .slice(0, 3)
    .map((e) => e.id),
};
```

- [ ] **Step 8.4: Run tests**

```bash
cd app/elohim-library && pnpm test -- projects/graphos/src/default/qahal/fixtures/canonical/cofc-congregation-sunday-morning.spec.ts 2>&1 | tail -20
```

Expected: PASS — 6 tests green. (Same fixture-content-gap recovery as Task 7.4 if anything fails.)

- [ ] **Step 8.5: Commit**

```bash
git add app/elohim-library/projects/graphos/src/default/qahal/fixtures/canonical/cofc-congregation-sunday-morning.ts \
        app/elohim-library/projects/graphos/src/default/qahal/fixtures/canonical/cofc-congregation-sunday-morning.spec.ts
git commit -m "feat(library-b): add CofC congregation Sunday-morning scene fixture

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 9: Hardins life-group Tuesday-evening scene fixture + spec

**Files:**
- Create: `.../canonical/hardins-life-group-tuesday-evening.ts` + `.spec.ts`

- [ ] **Step 9.1: Write the narrative-fidelity spec**

Create `hardins-life-group-tuesday-evening.spec.ts`:

```typescript
import { describe, expect, it } from 'vitest';
import { hardinsLifeGroupTuesdayEvening } from './hardins-life-group-tuesday-evening';

describe('hardins-life-group-tuesday-evening scene fixture', () => {
  const scene = hardinsLifeGroupTuesdayEvening;

  it('uses the life-group qahal id + archetype', () => {
    expect(scene.id).toBe('hardins-life-group');
    expect(scene.qahalArchetype).toBe('life-group');
  });

  it('binds the LIFE_GROUP_RUBRIC', () => {
    expect(scene.rubric.qahalId).toBe('hardins-life-group');
  });

  it('references John Hardin as host steward', () => {
    expect(scene.members.some((m) => m.id === 'john-hardin')).toBe(true);
  });

  it('references the Romans 12 v1 discussion in the stream', () => {
    expect(scene.streamEvents.some((e) => /romans 12.*1|verse 1/i.test(e.content))).toBe(true);
  });

  it('references hosting accumulation OR friction-gradient surfacing in stream OR co-steward observation', () => {
    const combined = scene.streamEvents.map((e) => e.content).join(' ') + scene.coStewardObservation;
    expect(combined).toMatch(/(host.*accum|twenty-three|friction-gradient|host.*tuesday)/i);
  });
});
```

- [ ] **Step 9.2: Run — fail**

```bash
cd app/elohim-library && pnpm test -- projects/graphos/src/default/qahal/fixtures/canonical/hardins-life-group-tuesday-evening.spec.ts 2>&1 | tail -10
```

Expected: FAIL — module missing.

- [ ] **Step 9.3: Author the fixture**

```typescript
/**
 * Canonical scene fixture — Hardins Life-Group, Tuesday evening.
 *
 * Grounded in storyteller canonical narrative §4.3.
 *
 * Named moments encoded:
 *   - 6 households present; group has been together 3 years
 *   - Romans 12 v1 discussion
 *   - Sarah's father in hospital
 *   - John Hardin's hosting accumulation gently caught by friction-gradient
 *   - The Lees offer to host next
 */

import type { Scene } from '../../../../designed/qahal/_lib/types';
import { LIFE_GROUP_FAMILIES } from '../primitives/mock-imagodei-profiles';
import { LIFE_GROUP_RUBRIC } from '../primitives/mock-rubrics';
import { LIFE_GROUP_TUESDAY_EVENING_STREAM } from '../primitives/mock-care-economy-events';
import { LIFE_GROUP_TOPOLOGY } from '../primitives/mock-social-compute-topology';

export const hardinsLifeGroupTuesdayEvening: Scene = {
  id: 'hardins-life-group',
  qahalIcon: '🪨',
  qahalLabel: 'Tuesday Life-Group',
  qahalArchetype: 'life-group',
  otherQahals: [
    { id: 'dowell-household', icon: '🏠', label: 'Dowell Household' },
    { id: 'cofc-congregation', icon: '⛪', label: 'Local Churches of Christ' },
    { id: 'wisdom-commons', icon: '🌳', label: 'Wisdom Commons' },
  ],
  rubric: LIFE_GROUP_RUBRIC,
  members: LIFE_GROUP_FAMILIES,
  streamEvents: LIFE_GROUP_TUESDAY_EVENING_STREAM,
  computeTopology: LIFE_GROUP_TOPOLOGY,
  coStewardObservation:
    'Three years of fellowship. John has hosted twenty-three of the last twenty-four Tuesdays — gently surfacing for the group to discuss.',
  curatedEprs: [
    { id: 'study-schedule', title: 'Study Schedule', provenance: 'curated-epr' },
    { id: 'hosting-rota', title: 'Hosting Rota', provenance: 'curated-epr' },
  ],
  externalLinks: [],
  pendingAcknowledgments: LIFE_GROUP_TUESDAY_EVENING_STREAM.filter((e) =>
    /sarah.*father|hospital|host.*offer/i.test(e.content)
  )
    .slice(0, 2)
    .map((e) => e.id),
};
```

- [ ] **Step 9.4: Run tests**

```bash
cd app/elohim-library && pnpm test -- projects/graphos/src/default/qahal/fixtures/canonical/hardins-life-group-tuesday-evening.spec.ts 2>&1 | tail -15
```

Expected: PASS — 5 tests green.

- [ ] **Step 9.5: Commit**

```bash
git add app/elohim-library/projects/graphos/src/default/qahal/fixtures/canonical/hardins-life-group-tuesday-evening.ts \
        app/elohim-library/projects/graphos/src/default/qahal/fixtures/canonical/hardins-life-group-tuesday-evening.spec.ts
git commit -m "feat(library-b): add Hardins life-group Tuesday-evening scene fixture

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 10: Wisdom commons Thursday-afternoon scene fixture + spec

**Files:**
- Create: `.../canonical/wisdom-commons-thursday-afternoon.ts` + `.spec.ts`

- [ ] **Step 10.1: Write the narrative-fidelity spec**

Create `wisdom-commons-thursday-afternoon.spec.ts`:

```typescript
import { describe, expect, it } from 'vitest';
import { wisdomCommonsThursdayAfternoon } from './wisdom-commons-thursday-afternoon';

describe('wisdom-commons-thursday-afternoon scene fixture', () => {
  const scene = wisdomCommonsThursdayAfternoon;

  it('uses the wisdom-commons qahal id + archetype', () => {
    expect(scene.id).toBe('wisdom-commons');
    expect(scene.qahalArchetype).toBe('wisdom-commons');
    expect(scene.qahalIcon).toBe('🌳');
  });

  it('references the Arkansas sister congregation', () => {
    expect(scene.streamEvents.some((e) => /arkansas/i.test(e.content))).toBe(true);
  });

  it('references peer council convening or REA reconciliation', () => {
    const combined = scene.streamEvents.map((e) => e.content).join(' ');
    expect(combined).toMatch(/(peer council|reconciliation|witness)/i);
  });

  it('includes the Arkansas elder as a peer member', () => {
    expect(scene.members.some((m) => m.id === 'arkansas-elder')).toBe(true);
  });

  it('binds the WISDOM_COMMONS_RUBRIC', () => {
    expect(scene.rubric.qahalId).toBe('wisdom-commons');
  });
});
```

- [ ] **Step 10.2: Run — fail**

```bash
cd app/elohim-library && pnpm test -- projects/graphos/src/default/qahal/fixtures/canonical/wisdom-commons-thursday-afternoon.spec.ts 2>&1 | tail -10
```

Expected: FAIL — module missing.

- [ ] **Step 10.3: Author the fixture**

```typescript
/**
 * Canonical scene fixture — Wisdom Commons, Thursday afternoon.
 *
 * Grounded in storyteller canonical narrative §4.4.
 *
 * Named moments encoded:
 *   - 83 participating congregations
 *   - Brother Cal's concern surface, submitted to the Arkansas sister congregation
 *   - Peer council convening; sits with the witness for two months
 *   - REA reconciliation event recorded when the congregation responds
 */

import type { Scene } from '../../../../designed/qahal/_lib/types';
import {
  COFC_ELDERS,
  ARKANSAS_SISTER_CONGREGATION_ELDER,
} from '../primitives/mock-imagodei-profiles';
import { WISDOM_COMMONS_RUBRIC } from '../primitives/mock-rubrics';
import { WISDOM_COMMONS_THURSDAY_STREAM } from '../primitives/mock-care-economy-events';
import { WISDOM_COMMONS_TOPOLOGY } from '../primitives/mock-social-compute-topology';

export const wisdomCommonsThursdayAfternoon: Scene = {
  id: 'wisdom-commons',
  qahalIcon: '🌳',
  qahalLabel: 'Wisdom Commons',
  qahalArchetype: 'wisdom-commons',
  otherQahals: [
    { id: 'dowell-household', icon: '🏠', label: 'Dowell Household' },
    { id: 'cofc-congregation', icon: '⛪', label: 'Local Churches of Christ' },
    { id: 'hardins-life-group', icon: '🪨', label: 'Tuesday Life-Group' },
  ],
  rubric: WISDOM_COMMONS_RUBRIC,
  members: [...COFC_ELDERS, ARKANSAS_SISTER_CONGREGATION_ELDER],
  streamEvents: WISDOM_COMMONS_THURSDAY_STREAM,
  computeTopology: WISDOM_COMMONS_TOPOLOGY,
  coStewardObservation:
    'The federation has 83 participating congregations. A concern surface from a sister congregation has been recorded; the peer council is convening.',
  curatedEprs: [
    { id: 'federation-rubric', title: 'Federation Rubric Template', provenance: 'curated-epr' },
    {
      id: 'peer-council-protocol',
      title: 'Peer Council Convening Protocol',
      provenance: 'curated-epr',
    },
  ],
  externalLinks: [],
  pendingAcknowledgments: WISDOM_COMMONS_THURSDAY_STREAM.filter((e) =>
    /arkansas|witness|council/i.test(e.content)
  )
    .slice(0, 2)
    .map((e) => e.id),
};
```

- [ ] **Step 10.4: Run tests**

```bash
cd app/elohim-library && pnpm test -- projects/graphos/src/default/qahal/fixtures/canonical/wisdom-commons-thursday-afternoon.spec.ts 2>&1 | tail -15
```

Expected: PASS — 5 tests green.

- [ ] **Step 10.5: Commit**

```bash
git add app/elohim-library/projects/graphos/src/default/qahal/fixtures/canonical/wisdom-commons-thursday-afternoon.ts \
        app/elohim-library/projects/graphos/src/default/qahal/fixtures/canonical/wisdom-commons-thursday-afternoon.spec.ts
git commit -m "feat(library-b): add Wisdom Commons Thursday-afternoon scene fixture

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 11: 7 variation fixtures (batch)

**Files:**
- Create: 7 `default/qahal/fixtures/variations/<name>.ts` + 7 `<name>.spec.ts`

Variations don't need narrative-fidelity tests — just typed-validity. The pattern repeats; each variation deviates from a canonical by changing one architectural dimension.

- [ ] **Step 11.1: Author the shared variation-spec template (reuse across all 7)**

The spec template for every variation:

```typescript
import { describe, expect, it } from 'vitest';
import type { Scene } from '../../../../designed/qahal/_lib/types';
import { /* fixture export name */ } from './FIXTURE_FILE';

describe('<FIXTURE_FILE> variation fixture', () => {
  const scene: Scene = /* fixture export name */;

  it('conforms to the Scene type contract', () => {
    expect(scene.id).toBeTruthy();
    expect(scene.qahalLabel).toBeTruthy();
    expect(scene.qahalIcon).toBeTruthy();
    expect(['household', 'congregation', 'life-group', 'wisdom-commons']).toContain(scene.qahalArchetype);
    expect(scene.rubric).toBeDefined();
    expect(Array.isArray(scene.members)).toBe(true);
    expect(Array.isArray(scene.streamEvents)).toBe(true);
    expect(scene.computeTopology).toBeDefined();
    expect(typeof scene.coStewardObservation).toBe('string');
    expect(Array.isArray(scene.curatedEprs)).toBe(true);
    expect(Array.isArray(scene.externalLinks)).toBe(true);
    expect(Array.isArray(scene.pendingAcknowledgments)).toBe(true);
  });
});
```

For each of the 7 variations below, create both `<name>.ts` and `<name>.spec.ts` (the spec uses the template above with the export name + fixture name substituted).

- [ ] **Step 11.2: Author `household-with-toddlers.ts`**

```typescript
/**
 * Variation — household with toddlers.
 * Edge case: stream dominated by repetitive small care events; member-ring shows
 * non-pilot tiers (toddlers as protected-tier child).
 */
import type { Scene } from '../../../../designed/qahal/_lib/types';
import { dowellHouseholdTuesdayMorning } from '../canonical/dowell-household-tuesday-morning';

export const householdWithToddlers: Scene = {
  ...dowellHouseholdTuesdayMorning,
  id: 'household-with-toddlers',
  qahalLabel: 'Household (toddlers)',
  members: dowellHouseholdTuesdayMorning.members.map((m) =>
    m.name.includes('James') ? { ...m, name: 'Toddler', bloomTier: 'remember' } : m
  ),
  coStewardObservation:
    'Care is mostly under-three care this week. The household is steady; the parents are tired.',
};
```

- [ ] **Step 11.3: Author `household-multi-generation.ts`**

```typescript
import type { Scene } from '../../../../designed/qahal/_lib/types';
import { dowellHouseholdTuesdayMorning } from '../canonical/dowell-household-tuesday-morning';

export const householdMultiGeneration: Scene = {
  ...dowellHouseholdTuesdayMorning,
  id: 'household-multi-generation',
  qahalLabel: 'Household (multi-generation)',
  coStewardObservation:
    'Three generations under one roof this week. Gertrude is here; James is here; care flows both directions.',
};
```

- [ ] **Step 11.4: Author `household-single-parent.ts`**

```typescript
import type { Scene } from '../../../../designed/qahal/_lib/types';
import { dowellHouseholdTuesdayMorning } from '../canonical/dowell-household-tuesday-morning';

export const householdSingleParent: Scene = {
  ...dowellHouseholdTuesdayMorning,
  id: 'household-single-parent',
  qahalLabel: 'Household (single parent)',
  members: dowellHouseholdTuesdayMorning.members.filter((m) => !m.name.includes('Jessica')),
  coStewardObservation:
    'One steward this season. The household is participating; the substrate has noted the load and is gently surfacing aid offers from the life-group.',
};
```

- [ ] **Step 11.5: Author `congregation-doctrinal-tension.ts`**

```typescript
import type { Scene } from '../../../../designed/qahal/_lib/types';
import { cofcCongregationSundayMorning } from '../canonical/cofc-congregation-sunday-morning';

export const congregationDoctrinalTension: Scene = {
  ...cofcCongregationSundayMorning,
  id: 'congregation-doctrinal-tension',
  qahalLabel: 'Congregation (doctrinal tension)',
  coStewardObservation:
    'A teaching from a sister congregation has surfaced concern among the elders. The peer-council mediation EPR is available.',
  curatedEprs: [
    ...cofcCongregationSundayMorning.curatedEprs,
    {
      id: 'dispute-mediation',
      title: 'Dispute Mediation EPR',
      provenance: 'curated-epr',
    },
  ],
};
```

- [ ] **Step 11.6: Author `congregation-newly-formed.ts`**

```typescript
import type { Scene } from '../../../../designed/qahal/_lib/types';
import { cofcCongregationSundayMorning } from '../canonical/cofc-congregation-sunday-morning';

export const congregationNewlyFormed: Scene = {
  ...cofcCongregationSundayMorning,
  id: 'congregation-newly-formed',
  qahalLabel: 'Congregation (newly formed)',
  streamEvents: cofcCongregationSundayMorning.streamEvents.slice(0, 3),
  coStewardObservation:
    'The congregation is in its first season. Cohesion thresholds are not yet reached; the substrate watches and waits.',
};
```

- [ ] **Step 11.7: Author `life-group-newly-formed.ts`**

```typescript
import type { Scene } from '../../../../designed/qahal/_lib/types';
import { hardinsLifeGroupTuesdayEvening } from '../canonical/hardins-life-group-tuesday-evening';

export const lifeGroupNewlyFormed: Scene = {
  ...hardinsLifeGroupTuesdayEvening,
  id: 'life-group-newly-formed',
  qahalLabel: 'Life-Group (newly formed)',
  streamEvents: hardinsLifeGroupTuesdayEvening.streamEvents.slice(0, 2),
  coStewardObservation:
    'Three meetings in. Vulnerability has not yet been offered. The substrate honors patience.',
};
```

- [ ] **Step 11.8: Author `wisdom-commons-reconciliation-recorded.ts`**

```typescript
import type { Scene } from '../../../../designed/qahal/_lib/types';
import { wisdomCommonsThursdayAfternoon } from '../canonical/wisdom-commons-thursday-afternoon';

export const wisdomCommonsReconciliationRecorded: Scene = {
  ...wisdomCommonsThursdayAfternoon,
  id: 'wisdom-commons-reconciliation-recorded',
  qahalLabel: 'Wisdom Commons (reconciliation recorded)',
  coStewardObservation:
    'The Arkansas sister congregation has written back. The teaching has been reconsidered. An REA reconciliation event has been recorded.',
};
```

- [ ] **Step 11.9: Author the 7 corresponding `.spec.ts` files using the template from 11.1**

Substitute the import name (e.g., `householdWithToddlers`) and the fixture file path for each. All 7 specs share the typed-validity assertions from step 11.1.

- [ ] **Step 11.10: Run all variation tests**

```bash
cd app/elohim-library && pnpm test -- projects/graphos/src/default/qahal/fixtures/variations/ 2>&1 | tail -25
```

Expected: PASS — 7 specs × ~10 typed-validity assertions = ~70 assertions green.

- [ ] **Step 11.11: Commit**

```bash
git add app/elohim-library/projects/graphos/src/default/qahal/fixtures/variations/
git commit -m "$(cat <<'EOF'
feat(library-b): add 7 variation scene fixtures across all 4 archetypes

3 household + 2 congregation + 1 life-group + 1 wisdom-commons variations.
Each spreads from a canonical fixture and tweaks one architectural
dimension (member composition, stream activity, co-steward observation,
or curated-EPR set). Typed-validity assertions cover the Scene contract;
narrative-fidelity assertions are intentionally absent — variations are
synthetic edge cases, not canonical scenes.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 12: Canonical story files (4)

**Files:**
- Create: `designed/qahal/homepage/__docs__/canonical/{dowell-household,cofc-congregation,hardins-life-group,wisdom-commons}.designed.stories.ts`

- [ ] **Step 12.1: Author `dowell-household.designed.stories.ts`**

```typescript
/**
 * Library B canonical pattern story — Dowell Household Tuesday-morning.
 *
 * Renders the storyteller §4.1 named moment: sick James, Sheila's soup,
 * Gertrude's check-in, "the household is steady."
 *
 * Default rendering: viewerTier='steward' (Matthew), lens='standard',
 * powerUserVisible=false, activePanel='stream'.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { qahalLightDecorator } from '../../../_lib/qahal-decorator';
import { renderQahalHomepage } from '../../../_lib/render-qahal-homepage';
import { dowellHouseholdTuesdayMorning } from '../../../../../default/qahal/fixtures/canonical/dowell-household-tuesday-morning';

const meta: Meta = {
  title: 'Designed/Qahal/Homepage/Canonical/Dowell Household',
  decorators: [qahalLightDecorator],
  parameters: {
    docs: {
      description: {
        component: `
The Tuesday-morning Dowell household scene from canonical narrative §4.1.

James is sick. Sheila sent chicken-and-rice soup the night before. Gertrude
checked in from half a continent away. Three pending acknowledgments wait
in the stream. The co-steward writes a quiet observation: "the household is steady."

Rendered through Matthew's steward lens at the standard capability profile —
the household's adult-pilot default view.
        `.trim(),
      },
    },
  },
};
export default meta;
type Story = StoryObj;

export const TuesdayMorning: Story = {
  render: () =>
    renderQahalHomepage(dowellHouseholdTuesdayMorning, {
      viewerTier: 'steward',
      powerUserVisible: false,
      lens: 'standard',
      activePanel: 'stream',
    }),
};
```

- [ ] **Step 12.2: Author `cofc-congregation.designed.stories.ts`**

```typescript
import type { Meta, StoryObj } from '@storybook/web-components';
import { qahalLightDecorator } from '../../../_lib/qahal-decorator';
import { renderQahalHomepage } from '../../../_lib/render-qahal-homepage';
import { cofcCongregationSundayMorning } from '../../../../../default/qahal/fixtures/canonical/cofc-congregation-sunday-morning';

const meta: Meta = {
  title: 'Designed/Qahal/Homepage/Canonical/CofC Congregation',
  decorators: [qahalLightDecorator],
  parameters: {
    docs: {
      description: {
        component: `
The Sunday-morning CofC congregation scene from canonical narrative §4.2.

230 members. Four elders — Brother Cal, Thompson, Davis, Rhodes — in plural
stewardship. Romans 12 sermon series in its fourth week. The youth retreat
needs two drivers. Three prayer requests. The co-steward notes the
congregation's reach into the neighborhood is rising slightly; three
life-groups are nearing the cohesion threshold.

Rendered through Brother Cal's steward lens at the standard capability profile.
        `.trim(),
      },
    },
  },
};
export default meta;
type Story = StoryObj;

export const SundayMorning: StoryObj = {
  render: () =>
    renderQahalHomepage(cofcCongregationSundayMorning, {
      viewerTier: 'steward',
      powerUserVisible: false,
      lens: 'standard',
      activePanel: 'stream',
    }),
};
```

- [ ] **Step 12.3: Author `hardins-life-group.designed.stories.ts`**

```typescript
import type { Meta, StoryObj } from '@storybook/web-components';
import { qahalLightDecorator } from '../../../_lib/qahal-decorator';
import { renderQahalHomepage } from '../../../_lib/render-qahal-homepage';
import { hardinsLifeGroupTuesdayEvening } from '../../../../../default/qahal/fixtures/canonical/hardins-life-group-tuesday-evening';

const meta: Meta = {
  title: 'Designed/Qahal/Homepage/Canonical/Hardins Life-Group',
  decorators: [qahalLightDecorator],
  parameters: {
    docs: {
      description: {
        component: `
The Tuesday-evening life-group scene from canonical narrative §4.3.

Six families. Three years of fellowship. Romans 12 verse 1 discussion.
Sarah's father is in hospital. The substrate has noticed John Hardin
hosted twenty-three of the last twenty-four Tuesdays — a gentle prompt
surfaces. The Lees offer to host next.

Rendered through John Hardin's steward lens at the standard capability profile.
        `.trim(),
      },
    },
  },
};
export default meta;
type Story = StoryObj;

export const TuesdayEvening: StoryObj = {
  render: () =>
    renderQahalHomepage(hardinsLifeGroupTuesdayEvening, {
      viewerTier: 'steward',
      powerUserVisible: false,
      lens: 'standard',
      activePanel: 'stream',
    }),
};
```

- [ ] **Step 12.4: Author `wisdom-commons.designed.stories.ts`**

```typescript
import type { Meta, StoryObj } from '@storybook/web-components';
import { qahalLightDecorator } from '../../../_lib/qahal-decorator';
import { renderQahalHomepage } from '../../../_lib/render-qahal-homepage';
import { wisdomCommonsThursdayAfternoon } from '../../../../../default/qahal/fixtures/canonical/wisdom-commons-thursday-afternoon';

const meta: Meta = {
  title: 'Designed/Qahal/Homepage/Canonical/Wisdom Commons',
  decorators: [qahalLightDecorator],
  parameters: {
    docs: {
      description: {
        component: `
The Thursday-afternoon wisdom commons scene from canonical narrative §4.4.

83 participating congregations. Brother Cal's congregation has been in
fellowship with the Arkansas sister congregation for thirty years. A
concern has surfaced from a recent teaching. The substrate convenes a
voluntary peer council; the council sits with the witness for two months.
The Arkansas congregation writes back: the teaching has been reconsidered.
An REA reconciliation event is recorded.

Rendered through Brother Cal's contributor lens at the standard capability profile.
        `.trim(),
      },
    },
  },
};
export default meta;
type Story = StoryObj;

export const ThursdayAfternoon: StoryObj = {
  render: () =>
    renderQahalHomepage(wisdomCommonsThursdayAfternoon, {
      viewerTier: 'contributor',
      powerUserVisible: false,
      lens: 'standard',
      activePanel: 'stream',
    }),
};
```

- [ ] **Step 12.5: Run the storybook test-runner on the new stories (smoke test)**

```bash
cd app/elohim-library && pnpm storybook --ci --quiet > /tmp/sb.log 2>&1 &
SB_PID=$!; sleep 30; pnpm exec test-storybook --url http://localhost:6006 --include "Designed/Qahal/Homepage/Canonical/**" 2>&1 | tail -30
kill $SB_PID
```

Expected: PASS — 4 stories render without runtime errors and pass the test-runner's a11y check.

- [ ] **Step 12.6: Commit**

```bash
git add app/elohim-library/projects/graphos/src/designed/qahal/homepage/__docs__/canonical/
git commit -m "$(cat <<'EOF'
feat(library-b): add 4 canonical Qahal-homepage pattern stories

Each renders one storyteller-canonical scene (§4.1 Dowell Tuesday, §4.2
CofC Sunday, §4.3 Hardins Tuesday, §4.4 wisdom-commons Thursday) via
the shared renderQahalHomepage composer and qahalLightDecorator. Each
story is ~30 lines: import scene fixture, import composer + decorator,
call render with steward/standard/no-power-user/stream defaults.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 13: Variation story files (7)

**Files:**
- Create: 7 `designed/qahal/homepage/__docs__/variations/<name>.designed.stories.ts`

Each variation story follows the same template as a canonical story; the only differences are: import the variation fixture instead, retitle.

- [ ] **Step 13.1: Author the variation-story template (reuse pattern)**

The shared template, with `<FIXTURE_NAME>` and `<FIXTURE_PATH>` and `<TITLE>` substituted per file:

```typescript
import type { Meta, StoryObj } from '@storybook/web-components';
import { qahalLightDecorator } from '../../../_lib/qahal-decorator';
import { renderQahalHomepage } from '../../../_lib/render-qahal-homepage';
import { <FIXTURE_NAME> } from '<FIXTURE_PATH>';

const meta: Meta = {
  title: 'Designed/Qahal/Homepage/Variations/<TITLE>',
  decorators: [qahalLightDecorator],
  parameters: {
    docs: {
      description: {
        component: '<one-paragraph note on what edge case this variation exercises>',
      },
    },
  },
};
export default meta;

export const Default: StoryObj = {
  render: () =>
    renderQahalHomepage(<FIXTURE_NAME>, {
      viewerTier: 'steward',
      powerUserVisible: false,
      lens: 'standard',
      activePanel: 'stream',
    }),
};
```

- [ ] **Step 13.2: Author the 7 variation story files**

| File | Title | Fixture import | Edge case note |
|---|---|---|---|
| `household-with-toddlers.designed.stories.ts` | "Household With Toddlers" | `householdWithToddlers` | Stream dominated by repetitive small-care events; ring shows child capability tier |
| `household-multi-generation.designed.stories.ts` | "Household Multi-Generation" | `householdMultiGeneration` | Three generations under one roof; co-steward observation reflects bidirectional care |
| `household-single-parent.designed.stories.ts` | "Household Single Parent" | `householdSingleParent` | One steward; substrate gently surfaces aid offers from the life-group |
| `congregation-doctrinal-tension.designed.stories.ts` | "Congregation Doctrinal Tension" | `congregationDoctrinalTension` | Concern surface from sister congregation; dispute-mediation EPR available |
| `congregation-newly-formed.designed.stories.ts` | "Congregation Newly Formed" | `congregationNewlyFormed` | First season; cohesion thresholds not yet reached |
| `life-group-newly-formed.designed.stories.ts` | "Life-Group Newly Formed" | `lifeGroupNewlyFormed` | Three meetings in; vulnerability not yet offered |
| `wisdom-commons-reconciliation-recorded.designed.stories.ts` | "Wisdom Commons Reconciliation Recorded" | `wisdomCommonsReconciliationRecorded` | Counterpoint to canonical concern-surface — the resolution moment |

For each file use the template above with the right import path:
- `../../../../../default/qahal/fixtures/variations/<fixture-file-name>` for the fixture import

- [ ] **Step 13.3: Run smoke test**

```bash
cd app/elohim-library && pnpm storybook --ci --quiet > /tmp/sb.log 2>&1 &
SB_PID=$!; sleep 30; pnpm exec test-storybook --url http://localhost:6006 --include "Designed/Qahal/Homepage/Variations/**" 2>&1 | tail -20
kill $SB_PID
```

Expected: PASS — 7 variation stories render cleanly.

- [ ] **Step 13.4: Commit**

```bash
git add app/elohim-library/projects/graphos/src/designed/qahal/homepage/__docs__/variations/
git commit -m "$(cat <<'EOF'
feat(library-b): add 7 variation Qahal-homepage pattern stories

3 household + 2 congregation + 1 life-group + 1 wisdom-commons.
Each demonstrates one architectural edge case (toddler-density,
multi-generation, single-steward, doctrinal-tension, first-season,
pre-vulnerability, post-reconciliation) using the shared composer.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 14: User-toggle story files (2)

**Files:**
- Create: `.../user-toggles/{simple-user-view,power-user-view}.designed.stories.ts`

- [ ] **Step 14.1: Author `simple-user-view.designed.stories.ts`**

```typescript
import type { Meta, StoryObj } from '@storybook/web-components';
import { qahalLightDecorator } from '../../../_lib/qahal-decorator';
import { renderQahalHomepage } from '../../../_lib/render-qahal-homepage';
import { dowellHouseholdTuesdayMorning } from '../../../../../default/qahal/fixtures/canonical/dowell-household-tuesday-morning';

const meta: Meta = {
  title: 'Designed/Qahal/Homepage/User Toggles/Simple User View',
  decorators: [qahalLightDecorator],
  parameters: {
    docs: {
      description: {
        component: `
The Dowell household homepage with **powerUserVisible: false** — the imagodei
setting "Power-user view" disabled (default for most operators).

The 4 power-user panels (standing-inspector, shefa-resources, attestations,
graph-discovery) are DOM-absent. The sidebar's power-user expandable section
is not rendered at all — per the UX spec discipline that this toggle is an
imagodei preference, not a homepage UX gesture.

Compare with the Power-User View story to see the same scene with the
toggle enabled.
        `.trim(),
      },
    },
  },
};
export default meta;
type Story = StoryObj;

export const Default: Story = {
  render: () =>
    renderQahalHomepage(dowellHouseholdTuesdayMorning, {
      viewerTier: 'steward',
      powerUserVisible: false,
      lens: 'standard',
      activePanel: 'stream',
    }),
};
```

- [ ] **Step 14.2: Author `power-user-view.designed.stories.ts`**

```typescript
import type { Meta, StoryObj } from '@storybook/web-components';
import { qahalLightDecorator } from '../../../_lib/qahal-decorator';
import { renderQahalHomepage } from '../../../_lib/render-qahal-homepage';
import { dowellHouseholdTuesdayMorning } from '../../../../../default/qahal/fixtures/canonical/dowell-household-tuesday-morning';

const meta: Meta = {
  title: 'Designed/Qahal/Homepage/User Toggles/Power User View',
  decorators: [qahalLightDecorator],
  parameters: {
    docs: {
      description: {
        component: `
The Dowell household homepage with **powerUserVisible: true** — the imagodei
setting "Power-user view" enabled.

The 4 power-user panels (standing-inspector, shefa-resources, attestations,
graph-discovery) appear in the sidebar's power-user-expandable section.
The chrome and core panels are otherwise identical to the Simple User View.

This story renders both the visible-stub power-user panels and the
established 5 deep-implementation panels of the simple tier — proving the
architecture's full surface from one toggle flip.
        `.trim(),
      },
    },
  },
};
export default meta;
type Story = StoryObj;

export const Default: Story = {
  render: () =>
    renderQahalHomepage(dowellHouseholdTuesdayMorning, {
      viewerTier: 'steward',
      powerUserVisible: true,
      lens: 'standard',
      activePanel: 'stream',
    }),
};
```

- [ ] **Step 14.3: Run smoke test**

```bash
cd app/elohim-library && pnpm storybook --ci --quiet > /tmp/sb.log 2>&1 &
SB_PID=$!; sleep 30; pnpm exec test-storybook --url http://localhost:6006 --include "Designed/Qahal/Homepage/User Toggles/**" 2>&1 | tail -15
kill $SB_PID
```

Expected: PASS — 2 stories render cleanly.

- [ ] **Step 14.4: Commit**

```bash
git add app/elohim-library/projects/graphos/src/designed/qahal/homepage/__docs__/user-toggles/
git commit -m "feat(library-b): add 2 user-toggle Qahal-homepage pattern stories

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 15: Capability-gating story files (5)

**Files:**
- Create: `.../capability-gating/{visitor-view,engaged-view,contributor-view,steward-view,protected-tier-view}.designed.stories.ts`

All five render the canonical Dowell scene; the only difference is `viewerTier`. Use the same template across them.

- [ ] **Step 15.1: Template** (substitute `<TIER>` and `<DESCRIPTION>` per file)

```typescript
import type { Meta, StoryObj } from '@storybook/web-components';
import { qahalLightDecorator } from '../../../_lib/qahal-decorator';
import { renderQahalHomepage } from '../../../_lib/render-qahal-homepage';
import { dowellHouseholdTuesdayMorning } from '../../../../../default/qahal/fixtures/canonical/dowell-household-tuesday-morning';

const meta: Meta = {
  title: 'Designed/Qahal/Homepage/Capability Gating/<HUMAN TITLE>',
  decorators: [qahalLightDecorator],
  parameters: {
    docs: {
      description: { component: '<DESCRIPTION>' },
    },
  },
};
export default meta;

export const Default: StoryObj = {
  render: () =>
    renderQahalHomepage(dowellHouseholdTuesdayMorning, {
      viewerTier: '<TIER>',
      powerUserVisible: false,
      lens: 'standard',
      activePanel: 'stream',
    }),
};
```

- [ ] **Step 15.2: Author all 5 files using the table**

| File | `<HUMAN TITLE>` | `<TIER>` | `<DESCRIPTION>` |
|---|---|---|---|
| `visitor-view.designed.stories.ts` | "Visitor View" | `'visitor'` | "Dowell household viewed by a visitor. External-link section is filtered via the co-steward (per the rubric's visitor visibility policy)." |
| `engaged-view.designed.stories.ts` | "Engaged View" | `'engaged'` | "Dowell household viewed by an engaged member. Full external-link visibility; power-user panels off." |
| `contributor-view.designed.stories.ts` | "Contributor View" | `'contributor'` | "Dowell household viewed by a contributor. Same external-link surface as engaged, with additional power-user-eligible affordances available via the imagodei settings palette." |
| `steward-view.designed.stories.ts` | "Steward View" | `'steward'` | "Dowell household viewed by a steward (Matthew). The default canonical view — rules + co-steward + social-compute panels are editable; external-links full." |
| `protected-tier-view.designed.stories.ts` | "Protected Tier View" | `'child'` | "Dowell household viewed by James (the household's child). External-link sidebar section is **DOM-absent** — per the household rubric's protected-tier discipline, James does not see external hyperlinks at all. The dignity-floor protection is visible." |

- [ ] **Step 15.3: Run smoke test**

```bash
cd app/elohim-library && pnpm storybook --ci --quiet > /tmp/sb.log 2>&1 &
SB_PID=$!; sleep 30; pnpm exec test-storybook --url http://localhost:6006 --include "Designed/Qahal/Homepage/Capability Gating/**" 2>&1 | tail -15
kill $SB_PID
```

Expected: PASS — 5 stories render cleanly.

- [ ] **Step 15.4: Commit**

```bash
git add app/elohim-library/projects/graphos/src/designed/qahal/homepage/__docs__/capability-gating/
git commit -m "$(cat <<'EOF'
feat(library-b): add 5 capability-gating Qahal-homepage pattern stories

visitor/engaged/contributor/steward/protected-tier views of the canonical
Dowell scene. The protected-tier story (James as child) demonstrates the
dignity-floor discipline: external-link sidebar section is DOM-absent.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 16: Playground story

**Files:**
- Create: `designed/qahal/homepage/__docs__/playground.designed.stories.ts`

- [ ] **Step 16.1: Author the playground**

```typescript
/**
 * Library B playground — interactive Qahal-homepage exploration.
 *
 * One story with Storybook argTypes controls for scene + tier + lens +
 * power-user + active-panel. Useful for stakeholder demos and the
 * recognition+distinction Checkpoint F verification.
 *
 * The 18 stable artifacts in this directory remain authoritative — this
 * playground complements them with interactive controls; it does not replace.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { qahalLightDecorator } from '../_lib/qahal-decorator';
import { renderQahalHomepage } from '../_lib/render-qahal-homepage';
import type { ActivePanel, Lens } from '../_lib/types';
import type { CapabilityTier } from '../../../default/qahal/fixtures/primitives/mock-imagodei-profiles';

import { dowellHouseholdTuesdayMorning } from '../../../default/qahal/fixtures/canonical/dowell-household-tuesday-morning';
import { cofcCongregationSundayMorning } from '../../../default/qahal/fixtures/canonical/cofc-congregation-sunday-morning';
import { hardinsLifeGroupTuesdayEvening } from '../../../default/qahal/fixtures/canonical/hardins-life-group-tuesday-evening';
import { wisdomCommonsThursdayAfternoon } from '../../../default/qahal/fixtures/canonical/wisdom-commons-thursday-afternoon';

const SCENE_MAP = {
  'dowell-household-tuesday-morning': dowellHouseholdTuesdayMorning,
  'cofc-congregation-sunday-morning': cofcCongregationSundayMorning,
  'hardins-life-group-tuesday-evening': hardinsLifeGroupTuesdayEvening,
  'wisdom-commons-thursday-afternoon': wisdomCommonsThursdayAfternoon,
} as const;

const CAPABILITY_TIERS: CapabilityTier[] = [
  'visitor',
  'engaged',
  'contributor',
  'steward',
  'elohim-support',
  'child',
  'idd_member',
  'elder_under_guardianship',
  'legal_steward_protected',
];

const LENSES: Lens[] = ['minimal', 'simple', 'standard', 'detail', 'debug', 'trace'];

const PANELS: ActivePanel[] = [
  'stream',
  'member-ring',
  'rules',
  'co-steward',
  'social-compute',
  'standing-inspector',
  'shefa-resources',
  'attestations',
  'graph-discovery',
];

interface PlaygroundArgs {
  sceneId: keyof typeof SCENE_MAP;
  viewerTier: CapabilityTier;
  powerUserVisible: boolean;
  lens: Lens;
  activePanel: ActivePanel;
}

const meta: Meta<PlaygroundArgs> = {
  title: 'Designed/Qahal/Homepage/Playground',
  decorators: [qahalLightDecorator],
  argTypes: {
    sceneId: {
      control: 'select',
      options: Object.keys(SCENE_MAP),
      description: 'Which canonical scene to render',
    },
    viewerTier: {
      control: 'select',
      options: CAPABILITY_TIERS,
      description: 'Capability tier of the viewer',
    },
    powerUserVisible: {
      control: 'boolean',
      description: 'Imagodei setting: power-user view enabled',
    },
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
  parameters: {
    docs: {
      description: {
        component: `
Interactive playground for the Qahal homepage architecture.

Use the controls panel to switch scenes, capability tiers, lenses, and
active panels. The Checkpoint F recognition+distinction test:
- Open Dowell + viewerTier=steward — observe James/Sheila/Gertrude
- Then drop viewerTier to 'child' — observe the external-link section vanish
        `.trim(),
      },
    },
  },
};
export default meta;
type Story = StoryObj<PlaygroundArgs>;

export const Interactive: Story = {
  render: (args) =>
    renderQahalHomepage(SCENE_MAP[args.sceneId], {
      viewerTier: args.viewerTier,
      powerUserVisible: args.powerUserVisible,
      lens: args.lens,
      activePanel: args.activePanel,
    }),
};
```

- [ ] **Step 16.2: Run smoke test**

```bash
cd app/elohim-library && pnpm storybook --ci --quiet > /tmp/sb.log 2>&1 &
SB_PID=$!; sleep 30; pnpm exec test-storybook --url http://localhost:6006 --include "Designed/Qahal/Homepage/Playground/**" 2>&1 | tail -15
kill $SB_PID
```

Expected: PASS — playground renders cleanly with the default args.

- [ ] **Step 16.3: Commit**

```bash
git add app/elohim-library/projects/graphos/src/designed/qahal/homepage/__docs__/playground.designed.stories.ts
git commit -m "feat(library-b): add interactive playground Qahal-homepage story

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 17: Pillar README + verification

**Files:**
- Create: `app/elohim-library/projects/graphos/src/designed/qahal/README.md`

- [ ] **Step 17.1: Author the pillar README**

```markdown
# designed/qahal — Library B Qahal-pillar pattern stories

Library B (the designed pattern library) for the Qahal pillar. Composes
Sprint 1A's elohim-qahal + elohim-imagodei Lit primitives into the
convergent Qahal homepage experience.

## Composition pattern

All 19 homepage stories share the same shape:

```typescript
import { qahalLightDecorator } from '../../../_lib/qahal-decorator';
import { renderQahalHomepage } from '../../../_lib/render-qahal-homepage';
import { someScene } from '../../../../../default/qahal/fixtures/{canonical,variations}/some-scene';

const meta: Meta = {
  title: 'Designed/Qahal/Homepage/.../...',
  decorators: [qahalLightDecorator],
};
export default meta;

export const Default: StoryObj = {
  render: () => renderQahalHomepage(someScene, {
    viewerTier: 'steward',
    powerUserVisible: false,
    lens: 'standard',
    activePanel: 'stream',
  }),
};
```

Every story file is ~30 lines. The composition logic lives once in
`_lib/render-qahal-homepage.ts`; the theme binding lives once in
`_lib/qahal-decorator.ts`.

## Adding a new homepage story

1. **Choose a fixture.** Canonical fixtures live at
   `default/qahal/fixtures/canonical/`; variations at
   `default/qahal/fixtures/variations/`. If you need a new fixture, add
   one there first (composing the existing `primitives/` mock-data
   modules) and add narrative-fidelity or typed-validity tests for it.
2. **Pick a bucket.** Four sidebar buckets exist:
   `canonical/`, `variations/`, `user-toggles/`, `capability-gating/`.
   Authors of new behavioral edge cases that need their own sidebar entry
   go in `variations/`; orthogonal toggles into `user-toggles/`; new
   viewer-tier rendering into `capability-gating/`. Choose by question:
   what does this story let a viewer recognize?
3. **Write the story file** at
   `homepage/__docs__/<bucket>/<name>.designed.stories.ts` using the
   template above.
4. **Run smoke test.** `pnpm storybook --ci --quiet` then
   `pnpm test-storybook --url http://localhost:6006 --include "Designed/Qahal/Homepage/<bucket>/**"`.

## Adding a new chrome element

If a new element is needed in the chrome assembly:

1. **Don't reach inside `renderQahalHomepage`** to add it inline. Update
   the composer module, adding the element to the appropriate column
   render function.
2. **Add a composer test** for the new element's presence + props in
   `_lib/render-qahal-homepage.spec.ts`.
3. **Bind its tokens** in `_lib/qahal-decorator.ts` light/dark/high-contrast
   blocks.

## Library boundary

Per `app/elohim-library/CLAUDE.md`: Library B never modifies primitives'
CSS, JSDoc, tag names, or behavior. If you need a `@cssprop` that doesn't
exist on a primitive, raise a `component-architect` follow-up — don't
reach inside the element.

## Cross-references

- Design spec: `genesis/docs/superpowers/specs/2026-05-22-sprint-1b-library-b-design.md`
- Implementation plan: `genesis/docs/superpowers/plans/2026-05-22-sprint-1b-library-b-pattern-stories.md`
- UX design spec: `genesis/docs/superpowers/specs/2026-05-22-qahal-homepage-ux-design.md`
- Canonical narratives: `genesis/docs/superpowers/specs/2026-05-21-qahal-section-4-canonical-narratives.md`
- Library A elements: `app/elohim-elements/elohim-qahal/`, `app/elohim-elements/elohim-imagodei/`
- Library boundary doctrine: `app/elohim-library/CLAUDE.md`
```

- [ ] **Step 17.2: Run the full library test suite**

```bash
cd app/elohim-library && pnpm test 2>&1 | tail -30
```

Expected: PASS — all composer + decorator + 4 canonical + 7 variation specs green. Total ~50-70 new tests; pre-existing tests still pass.

- [ ] **Step 17.3: Run the storybook test-runner against all new stories**

```bash
cd app/elohim-library && pnpm storybook --ci --quiet > /tmp/sb.log 2>&1 &
SB_PID=$!; sleep 30; pnpm exec test-storybook --url http://localhost:6006 --include "Designed/Qahal/Homepage/**" 2>&1 | tail -30
kill $SB_PID
```

Expected: PASS — 19 stories render cleanly with no runtime errors and clean a11y.

- [ ] **Step 17.4: Manual Checkpoint F verification (operator-judgment)**

Start Storybook locally:
```bash
cd app/elohim-library && pnpm storybook
```

Open the browser and navigate to:
- `Designed/Qahal/Homepage/Canonical/Dowell Household` — verify a non-technical observer recognizes the storyteller's Tuesday-morning scene (James + Sheila's soup + Gertrude's check-in + "household is steady")
- `Designed/Qahal/Homepage/Capability Gating/Protected Tier View` — verify the external-link sidebar section is absent for James-as-child (dignity-floor protection visible)
- `Designed/Qahal/Homepage/Playground` — flip `viewerTier` from `steward` to `child` and watch the external-link section vanish; flip `sceneId` between the four canonical scenes and observe the chrome carries each archetype

Record the operator judgment in the PR description or a follow-up artifact.

- [ ] **Step 17.5: Commit**

```bash
git add app/elohim-library/projects/graphos/src/designed/qahal/README.md
git commit -m "$(cat <<'EOF'
docs(library-b): add qahal-pillar README with composition + extension guide

Documents the renderQahalHomepage + qahalLightDecorator pattern for
future authors. Explains how to add a new homepage story, a new chrome
element, and reinforces the Library B / Library A boundary discipline
from app/elohim-library/CLAUDE.md.

Closes Sprint 1B.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Coverage summary

After all 17 tasks complete, the sprint produces:

| Surface | Count | Tests |
|---|---|---|
| `_lib/` modules | 4 files (types, decorator, composer, plus 2 spec files) | ~25 composer + ~5 decorator tests |
| Canonical scene fixtures | 4 (one per worked example) | 11 + 6 + 5 + 5 = 27 narrative-fidelity assertions |
| Variation scene fixtures | 7 (across all 4 archetypes) | ~70 typed-validity assertions |
| Story files | 19 (4 canonical + 7 variations + 2 toggle + 5 capability-gating + 1 playground) | smoke-tested by `test-storybook` |
| Documentation | 1 pillar README | — |
| Path migration | 4 fixture modules relocated to `primitives/` | — |
| Vitest config | 1 include-pattern addition | — |

Total commits: ~17. Total new test cases: ~125. Total new story entries in Storybook sidebar: 19.

All work conforms to the Library A / Library B boundary discipline from `app/elohim-library/CLAUDE.md`: no primitive is modified; all binding happens at the story-decorator level; all data composes the three sources of truth (ts-rs view types, manifest vocabulary, graphos brand tokens).
