# Unified Design Surface — Narrative Scaffold (Sprint 1)

**Date**: 2026-05-04
**Status**: Draft — pending implementation plan
**Scope**: Stand up the Storybook navigation IA for the Elohim Protocol's unified design surface, and auto-import existing genesis narrative (manifesto, brand spec, vocabulary, governance docs, A2O `.feature` files) into that IA via thin MDX wrappers. **No hand-authored prose.** Empty domain pillar scaffolds prepared to receive components in subsequent sprints.

**Builds on**: `2026-05-04-storybook-deployment-design.md` (deployed Storybook 10 to `storybook.elohim.host`).

**Defers**: component migration from `elohim-app`/`doorway-app` (sprint 2), pattern library re-import to align handrolled implementations with unified schema (sprint 3), `@storybook/addon-mcp` wiring (later sprint), `frontend-designer` subagent contract (later sprint).

---

## Motivation

Storybook is now deployed but presently shows only five diagram primitives from `lamad-ui`. The unified design surface vision — narrative + foundations + opinionated domain reference implementations + components, all coherent — needs the IA stood up *before* component migration begins. Otherwise migration produces an unstructured pile.

This sprint's deliverable is the **organized empty surface**: a designer or contributor opening Storybook sees the full shape of the protocol (Why → What → How → Foundations → Domains → Reference) populated with imported genesis narrative, with empty component slots clearly marked for future sprints. The narrative is read-only for now; it lives in `genesis/` and Storybook projects it.

The architecture pattern here — Foundations as shared substrate, Domains as opinionated reference implementations — mirrors how Material Design substrates Google's Gmail / Drive / Docs / News / YouTube as co-equal expressions of one design language. Identity (Imagodei), Learning (Lamad), Community (Qahal), Economy (Shefa), and Doorway are the five reference implementations of the protocol's design substrate.

The design system itself is **graphos** — the elohim-protocol-native design language seeded by `genesis/graphos/` (vocabulary register + brand spec) and grown into a navigable surface inside Storybook. The new library project is named `graphos` to make the concept explicit at the codebase level.

### Naming: graphos is the brand, Storybook is (currently) the engine

The surface being built **is graphos**. Storybook 10 is its current rendering technology, not its identity. Treat "Storybook" as the render engine the way an Angular app treats Webpack — load-bearing today, replaceable tomorrow. Eventually graphos may migrate off Storybook to elohim-native rendering (likely an Angular shell + Holochain content plumbing that consumes the same MDX/wrapper pattern). The MDX wrappers and `sync-genesis.mjs` are designed to survive that migration — they're not Storybook-coupled beyond the `<Meta>` and `<Markdown>` blocks.

**Deploy target stays at `storybook.elohim.host` for sprint 1.** Hostname migration is not in scope and not a near-term concern.

For sprint 1, the Storybook page title (`manager.ts` theme) and any user-visible chrome should read "graphos" rather than the default Storybook branding. This is the cheapest signal that the rename is real — pure cosmetic, no infra change.

## Information architecture

The Storybook sidebar surfaces four top-level sections plus per-domain sub-trees:

```
I. NARRATIVE FLOW   (designer's reading path)
   Why /
     Manifesto                    ← genesis/genesis/docs/content/elohim-protocol/manifesto.md
     Constitution                 ← .../constitution.md
     Vision                       ← .../global-orchestra.md
   What /
     Brand                        ← genesis/graphos/elohim-protocol-design-spec.md
                                    (Voice + Design Principles render as
                                    anchored sections within Brand for now;
                                    future split if/when they become
                                    standalone genesis files)
   How /
     Protocol Specification       ← .../protocol-specification.md
     Governance Layers            ← .../governance-layers-architecture.md
     EPR Developer Guide          ← .../epr-developer-guide.md
     Hardware Spec                ← .../hardware-spec.md

II. FOUNDATIONS     (elohim-core — shared substrate every domain speaks)
    Vocabulary Register           ← genesis/graphos/vocabulary.md
    EPR Elements                  ← placeholder (filled in later spec)
    REA Primitives                ← placeholder (filled in later spec)
    Brand Atoms                   ← placeholder (filled in later spec)
    Component Atoms               ← placeholder (sprint 2 populates)

III. DOMAINS        (opinionated reference implementations)

    Identity (Imagodei) /
      Stories                     ← genesis/a2o/features/auth/*.feature
      Reference Design            ← placeholder (no single-file source in genesis yet)
      Components                  ← placeholder ("arrives in sprint 2")
    Learning (Lamad) /
      Stories                     ← genesis/a2o/features/lamad/*.feature
                                    + genesis/a2o/features/content/*.feature
      Reference Design            ← genesis/genesis/docs/content/elohim-protocol/lamad.md
                                    (value_scanner/ directory pointer in
                                    placeholder; needs single-file synthesis
                                    or sprint-2 directory-source support)
      Components                  ← placeholder
    Community (Qahal) /
      Stories                     ← genesis/a2o/features/qahal/*.feature
      Reference Design            ← placeholder (governance/, governance_layers/
                                    are directory-sourced; mapping table
                                    supports single files only in sprint 1)
      Components                  ← placeholder
    Economy (Shefa) /
      Stories                     ← genesis/a2o/features/shefa/*.feature
      Reference Design            ← placeholder (economic_coordination/ is
                                    directory-sourced; same constraint)
      Components                  ← placeholder
    Doorway /
      Stories                     ← genesis/a2o/features/delivery/*.feature
                                    + genesis/a2o/features/browser/*.feature
      Reference Design            ← placeholder
      Components                  ← placeholder

IV. REFERENCE       (dev-facing — bottom of nav)
    Federation                    ← genesis/a2o/features/federation/*.feature
    Resilience                    ← genesis/a2o/features/resilience/*.feature
    Deployment                    ← genesis/a2o/features/deployment/*.feature
    Cross-cutting Stories         ← genesis/a2o/features/elohim/*.feature
    Specs                         ← genesis/docs/superpowers/specs/*.md (subset)
```

**Naming rule.** Domain top-levels are bilingual: "Identity (Imagodei)", "Learning (Lamad)", etc. — teaches protocol vocabulary to designers in context without burying English. Section I (Narrative Flow) titles stay accessible English only, honoring the existing rule that `genesis/docs/content/elohim-protocol/` narrative uses accessible English.

**Where `autonomous_entity`, `public_observer`, `social_medium`, `observer-protocol.md` land.** These sit under `IV. Reference` for now (best-effort triage). The path-mapping table in `sync-genesis.mjs` is a single source of truth and easy to revise as classifications firm up.

## Repo layout

**New project**: `app/elohim-library/projects/graphos/`. Distinct from `lamad-ui` (which stays the component library). The narrative scaffold has different concerns than the component library — keeping them separate prevents the eventual lamad-ui rename / pillar-split from churning narrative wrappers.

```
app/elohim-library/
  projects/
    graphos/                          (NEW)
      ng-package.json                       (not published; storybook-only)
      tsconfig.lib.json
      src/
        public-api.ts                       (empty re-export)
        narrative/
          why/
            __docs__/
              manifesto.mdx                 ← thin wrapper
              constitution.mdx
              vision.mdx
          what/
            __docs__/
              brand.mdx
              voice.mdx
              design-principles.mdx
          how/
            __docs__/
              protocol-specification.mdx
              governance-layers.mdx
              epr-developer-guide.mdx
              hardware-spec.mdx
        foundations/
          __docs__/
            vocabulary-register.mdx
            epr-elements.mdx                (placeholder)
            rea-primitives.mdx              (placeholder)
            brand-atoms.mdx                 (placeholder)
            component-atoms.mdx             (placeholder)
        domains/
          identity/
            __docs__/
              index.mdx                     (landing)
              stories.mdx                   (auto-render of feature files)
              reference.mdx                 (placeholder)
              components.mdx                (placeholder)
          learning/
          community/
          economy/
          doorway/
        reference/
          __docs__/
            federation.mdx
            resilience.mdx
            deployment.mdx
            cross-cutting-stories.mdx
            specs-index.mdx
        components/
          feature-file.component.ts         ← gherkin renderer
          markdown-embed.component.ts       ← thin wrapper around Storybook Markdown
        imported/                           (gitignored — output of sync-genesis.mjs)
          .gitignore                        (* / !.gitignore)

  scripts/
    sync-genesis.mjs                        (NEW — pre-build copy + path mapping)
```

**Modified files**:

- `app/elohim-library/.storybook/main.ts` — add `projects/graphos/src/**/__docs__/**/*.@(stories.ts|mdx)` to glob (already covered by existing `projects/**/__docs__/**` pattern; no edit unless needed).
- `app/elohim-library/package.json` — add `prebuild:storybook` and `prestart:storybook` to invoke `sync-genesis.mjs`.
- `app/elohim-library/angular.json` — register the new `graphos` project.
- `genesis/orchestrator/build-graph/...` — extend `elohim-storybook.changePatterns` to cover genesis content paths.

## Design

### MDX wrapper pattern

Every imported page is a 3-5 line MDX file. Example (`narrative/why/__docs__/manifesto.mdx`):

```mdx
import { Meta, Markdown } from '@storybook/blocks';
import content from '../../../imported/manifesto.md?raw';

<Meta title="I. Why / Manifesto" />

<Markdown>{content}</Markdown>
```

The wrapper does three things only: declares the Storybook title (driving sidebar IA), imports the synced content as a raw string, and renders it. No hand-authored body.

Caveat: domain `index.mdx` files include a one-line caption above the auto-link list (e.g., "Identity is the Imagodei pillar — presence, profile, recovery, capabilities."). This is the only authored prose in sprint 1, and it's strictly framing, not design narrative. Captions live as constants in `sync-genesis.mjs` so they stay in one place.

### Genesis sync mechanism

`scripts/sync-genesis.mjs` is the single source of truth for the genesis-to-storybook mapping. It runs as `prebuild` and `prestart` hooks. Output goes to `projects/graphos/imported/`, which is gitignored.

Responsibilities:

1. **Copy** mapped genesis files into `imported/` with deterministic filenames.
2. **Render** each `.feature` file through a thin gherkin-to-markdown transform (or copy raw if the `<FeatureFile>` component handles syntax highlighting itself — see "Gherkin rendering" below; either choice is fine, only one survives the implementation plan).
3. **Generate** auto-populated index pages where the IA calls for "all features in dir X as link list."
4. **Validate** that every entry in the path-mapping table resolves to an existing genesis file; fail the build with a clear error otherwise (catches genesis renames before they silently break Storybook).

The script is idempotent and produces no output to stdout under success. Failures print the missing path and the IA slot it was meant to fill.

### Path-mapping table

Encoded as a constant in `sync-genesis.mjs`. Spec author's best-effort mapping; revisable without spec amendment. Implementation plan should review and refine before coding starts.

```js
const MAPPINGS = [
  // I. Narrative Flow / Why
  { from: 'genesis/docs/content/elohim-protocol/manifesto.md',
    to: 'narrative/why/manifesto.md',
    title: 'I. Why / Manifesto' },
  { from: 'genesis/docs/content/elohim-protocol/constitution.md',
    to: 'narrative/why/constitution.md',
    title: 'I. Why / Constitution' },
  { from: 'genesis/docs/content/elohim-protocol/global-orchestra.md',
    to: 'narrative/why/vision.md',
    title: 'I. Why / Vision' },
  // I. Narrative Flow / What
  { from: 'graphos/elohim-protocol-design-spec.md',
    to: 'narrative/what/brand.md',
    title: 'I. What / Brand' },
  // I. Narrative Flow / How
  { from: 'docs/content/elohim-protocol/protocol-specification.md',
    to: 'narrative/how/protocol-specification.md',
    title: 'I. How / Protocol Specification' },
  { from: 'genesis/docs/content/elohim-protocol/governance-layers-architecture.md',
    to: 'narrative/how/governance-layers.md',
    title: 'I. How / Governance Layers' },
  { from: 'docs/content/elohim-protocol/epr-developer-guide.md',
    to: 'narrative/how/epr-developer-guide.md',
    title: 'I. How / EPR Developer Guide' },
  { from: 'genesis/docs/content/elohim-protocol/hardware-spec.md',
    to: 'narrative/how/hardware-spec.md',
    title: 'I. How / Hardware Spec' },
  // II. Foundations
  { from: 'graphos/vocabulary.md',
    to: 'foundations/vocabulary-register.md',
    title: 'II. Foundations / Vocabulary Register' },
  // III. Domains — globs (each glob produces one MDX per match)
  { fromGlob: 'a2o/features/auth/*.feature',
    toDir: 'domains/identity/stories/',
    titleFn: (name) => `III. Domains / Identity (Imagodei) / Stories / ${name}` },
  { fromGlob: 'a2o/features/lamad/*.feature',
    toDir: 'domains/learning/stories/',
    titleFn: (name) => `III. Domains / Learning (Lamad) / Stories / ${name}` },
  { fromGlob: 'a2o/features/content/*.feature',
    toDir: 'domains/learning/stories/',
    titleFn: (name) => `III. Domains / Learning (Lamad) / Stories / ${name}` },
  { fromGlob: 'a2o/features/qahal/*.feature',
    toDir: 'domains/community/stories/',
    titleFn: (name) => `III. Domains / Community (Qahal) / Stories / ${name}` },
  { fromGlob: 'a2o/features/shefa/*.feature',
    toDir: 'domains/economy/stories/',
    titleFn: (name) => `III. Domains / Economy (Shefa) / Stories / ${name}` },
  { fromGlob: 'a2o/features/delivery/*.feature',
    toDir: 'domains/doorway/stories/',
    titleFn: (name) => `III. Domains / Doorway / Stories / ${name}` },
  { fromGlob: 'a2o/features/browser/*.feature',
    toDir: 'domains/doorway/stories/',
    titleFn: (name) => `III. Domains / Doorway / Stories / ${name}` },
  // Domain Reference Design (where genesis content exists)
  { from: 'genesis/docs/content/elohim-protocol/lamad.md',
    to: 'domains/learning/reference.md',
    title: 'III. Domains / Learning (Lamad) / Reference Design' },
  // IV. Reference
  { fromGlob: 'a2o/features/federation/*.feature',
    toDir: 'reference/federation/',
    titleFn: (name) => `IV. Reference / Federation / ${name}` },
  { fromGlob: 'a2o/features/resilience/*.feature',
    toDir: 'reference/resilience/',
    titleFn: (name) => `IV. Reference / Resilience / ${name}` },
  { fromGlob: 'a2o/features/deployment/*.feature',
    toDir: 'reference/deployment/',
    titleFn: (name) => `IV. Reference / Deployment / ${name}` },
  { fromGlob: 'a2o/features/elohim/*.feature',
    toDir: 'reference/cross-cutting/',
    titleFn: (name) => `IV. Reference / Cross-cutting Stories / ${name}` },
];
```

Genesis files not in the mapping are *intentionally not surfaced* in sprint 1 (e.g., `autonomous_entity/`, `public_observer/`, `social_medium/`, `observer-protocol.md`, `value_scanner/`). Adding them is a one-line edit to `MAPPINGS` and is expected as the IA matures.

### Gherkin rendering

Two acceptable approaches; implementation plan picks one:

**(a) Pre-render to markdown.** `sync-genesis.mjs` transforms `.feature` to GitHub-flavored markdown (with a code fence for syntax highlight). The MDX wrapper just `<Markdown>`s it. Simple, reuses the markdown embed component, no new Angular component needed.

**(b) `<FeatureFile>` Angular component.** A real Angular component renders the gherkin AST with proper styling (Feature/Scenario/Given/When/Then visual hierarchy). MDX wrapper imports + uses the component.

Recommendation: **(a) for sprint 1** — ships faster, proves the IA, doesn't block on visual design. **(b) is a sprint-2 polish task** once we know what designers want from the gherkin view.

### Build pipeline

Storybook config is unchanged — the existing glob `projects/**/__docs__/**/*.@(stories.ts|mdx)` already covers the new project. Nothing in `.storybook/main.ts` needs editing.

The `prebuild` and `prestart` scripts gain a step:

```json
{
  "scripts": {
    "prestart": "node scripts/sync-genesis.mjs",
    "prebuild": "node scripts/sync-genesis.mjs && pnpm --filter elohim-library build-storybook"
  }
}
```

(Existing `prebuild` may already chain — implementation plan reconciles.)

Vite raw imports (`.md?raw`, `.feature?raw`) are supported by Storybook 10's Angular framework (uses Vite under the hood). No additional config needed.

### Build trigger broadening (orchestrator)

Per memory, build-manifest.json + graph-walker authority is required. The `elohim-storybook` pipeline's change patterns must extend to genesis content:

```yaml
elohim-storybook:
  changePatterns:
    - app/elohim-library/**          # already there
    - genesis/docs/content/elohim-protocol/**    # NEW
    - genesis/graphos/**                          # NEW
    - genesis/a2o/features/**                     # NEW
    - genesis/docs/superpowers/specs/**           # NEW (only if sprint surfaces specs)
```

**Trigger fanout risk**: genesis is edited far more frequently than `app/elohim-library`. A naive add fans out to many storybook rebuilds per day. Mitigation: pipeline already has `pnpm install` / `pnpm build-storybook` cached layers; rebuild cost is dominated by Vite bundling, which is fast (<2 min on a fresh tree). Acceptable for sprint 1; if it becomes painful, narrow the pattern list (e.g., glob to specific files in the mapping table) in a follow-up.

### Domain landing page (`domains/identity/__docs__/index.mdx`)

Pattern (one-line caption + auto-rendered link list — captions are the only authored prose):

```mdx
import { Meta } from '@storybook/blocks';

<Meta title="III. Domains / Identity (Imagodei)" />

# Identity (Imagodei)

The protocol's **identity** reference implementation — presence, profile, recovery, capabilities, and the trust contracts that bind agents to their commitments.

## Stories
- [Auth flows](?path=/docs/iii-domains-identity-imagodei-stories--docs)

## Reference Design
*Pending — see `genesis/docs/content/elohim-protocol/` for source materials.*

## Components
*Component documentation arrives in sprint 2 (component migration from `elohim-app/imagodei/`).*
```

The landing-page caption is the *one* place sprint 1 admits authored prose. All five caption strings live in `sync-genesis.mjs` constants so they're discoverable, reviewable, and easy to revise.

## Risks

1. **Path-mapping decay.** Genesis renames silently break Storybook. Mitigation: `sync-genesis.mjs` validates every mapping entry resolves to an existing file; build fails loudly otherwise.
2. **Trigger fanout.** Storybook rebuilds on most genesis edits. Acceptable cost for sprint 1; revisit if painful.
3. **IA classification disputes.** "Where does `social_medium/` live?" — sprint 1 punts most ambiguous content to `IV. Reference` or omits it. Iteration is cheap (one mapping table edit). Don't litigate every classification before shipping the scaffold.
4. **MDX/Vite raw-import behavior under Storybook 10.** Confirmed supported, but untested in this repo. Implementation plan should validate with one wrapper before scaling.
5. **Foundations placeholders feel hollow.** EPR Elements / REA Primitives / Brand Atoms / Component Atoms slots will look empty with "coming soon" content for a while. Trade-off: shipping the IA shape now is more valuable than waiting for Foundations content to be authored. Each placeholder carries a one-liner pointing to where the eventual content will be sourced.
6. **Doorway has no a2o auth coverage.** `genesis/a2o/features/auth/` is mapped to Identity, but doorway also surfaces auth (recovery, login). If/when auth scenarios split between identity-domain and doorway-projection, the mapping table revises.

## Out of scope (explicitly)

- **Component migration** from `elohim-app` (~178 components) or `doorway-app` (~64 components). Sprint 2.
- **Pattern library re-import**: rewriting handrolled implementations to consume migrated components. Sprint 3.
- **Pattern library reorg**: renaming `lamad-ui` to `elohim-ui` or splitting into `imagodei-ui` / `qahal-ui` / etc. Sprint 2 or 3.
- **`@storybook/addon-mcp` wiring** for backend agent access. Later sprint.
- **`frontend-designer` subagent contract** (CLAUDE.md updates, MCP exposure surface, authoring affordances). Later sprint.
- **EPR Elements / REA Primitives / Brand Atoms** Foundations content. Their own specs.
- **Gherkin execution status** (linking `.feature` files to CI pass/fail). Later sprint.
- **Custom narrative authoring** (designers writing native MDX content as opposed to embedded genesis files). Not contemplated; if it becomes desired, it gets its own spec.

## Success criteria

The sprint is done when:

1. Visiting `storybook.elohim.host` shows the four-section IA in the sidebar with all narrative pages (Why / What / How / Foundations / Domains-Stories / Reference) populated by imported genesis content.
2. Editing a genesis file (e.g., `manifesto.md`) and pushing triggers a Storybook rebuild that surfaces the new content.
3. Each domain landing page shows its stories link list, a placeholder for reference design (where no genesis content is mapped), and the explicit "Components arrive in sprint 2" message.
4. Foundations placeholders for EPR / REA / Brand Atoms / Component Atoms are present with their "pending" markers and source pointers.
5. `sync-genesis.mjs` validates the mapping table and fails loudly on missing source files.
6. No hand-authored design narrative has been written. Only IA titles, link list captions, and one-line domain framing strings (in `sync-genesis.mjs` constants).
