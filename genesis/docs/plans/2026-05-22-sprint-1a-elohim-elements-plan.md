# Sprint 1A — elohim-elements Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Author all Library A elohim-elements for the Qahal homepage MVP — qahal-pillar primitives, chrome, panels (5 deep-impl + 4 visual-stub), resource list sidebar sections, plus imagodei-pillar settings palette + introspection elements. Includes shared mock-data primitives that downstream Library B stories will consume. Produces a self-contained, testable element library with capability profiles, three precondition gates (a11y + i18n + ua-prefs), and minimal default story coverage.

**Architecture:** Per-pillar Lit web component packages (`elohim-qahal`, `elohim-imagodei`) following the elohim-core canonical pattern. Each element: `.ts` source + `.spec.ts` behavior+a11y+i18n+ua-prefs tests + `.manifest.spec.ts` Custom Elements Manifest contract test. CapabilityAwareElement mixin from elohim-core provides the capability-profile JSDoc parsing. Mock-data primitives live in graphos library default/qahal directory; Library A default stories live alongside.

**Tech Stack:** Lit 3.x, TypeScript, Vite, `@open-wc/testing` + `web-test-runner`, axe-core, `@custom-elements-manifest/analyzer`, pnpm workspaces, Storybook for Web Components.

**Companion documents:**
- UX design spec: `/projects/elohim/genesis/docs/superpowers/specs/2026-05-22-qahal-homepage-ux-design.md`
- Vision spec: `/projects/elohim/genesis/docs/superpowers/specs/2026-05-21-qahal-architecture-vision.md`
- Roadmap: `/projects/elohim/genesis/docs/plans/2026-05-21-qahal-mvp-roadmap.md`
- Canonical reference element: `app/elohim-elements/elohim-core/src/elohim-button.ts`

**Out of scope (deferred to Plan B):** Library B designed pattern stories, canonical/variation fixtures, user-toggle stories, capability-gating stories, Storybook 0.0.0.0-binding configuration. This plan stops at "elements work + render in their own default stories." Plan B composes them into the convergent homepage demonstration.

---

## Conventions used throughout this plan

### Lit element pattern (canonical)

Every element follows the elohim-core pattern:

```ts
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';
import { CapabilityAwareElement } from '@elohim/elohim-core/capability';

/**
 * Brief description of what this element is.
 *
 * @element elohim-qahal-foo
 * @prop {Type} propName - Description
 * @slot - Default slot description
 * @cssprop --elohim-qahal-foo-bg - Override background
 * @csspart container - The internal container
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings visitor | engaged | contributor | steward | elohim-support
 * @capabilityContentCertainty not-observed
 * @capabilityStates empty:supported, loading:supported, error:supported, stale:supported, contested:n/a, offline:supported, unauthorized:supported
 */
export class ElohimQahalFoo extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`/* ... */`;
  @property({ type: String }) propName = 'default';
  override render() { return html`<!-- ... -->`; }
}
```

### Test pattern (`.spec.ts`)

```ts
import { aTimeout, elementUpdated, expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import './register.js';
import type { ElohimQahalFoo } from './elohim-qahal-foo.js';
import { clearMediaQueries, measureLuminanceChanges } from '@elohim/elohim-core/testing/ua-prefs';
import { renderInLocale, requiresLogicalProperties } from '@elohim/elohim-core/testing/i18n';

describe('<elohim-qahal-foo>', () => {
  // Rendering tests
  it('renders default content', async () => { /* ... */ });
  // Property tests
  it('reflects propName to attribute', async () => { /* ... */ });
  // a11y tests (axe-core)
  it('passes axe accessibility audit', async () => { /* ... */ });
  // i18n tests
  it('renders correctly in rtl locale', async () => { /* ... */ });
  // ua-prefs tests
  it('respects prefers-reduced-motion', async () => { /* ... */ });
});
```

### Manifest spec pattern (`.manifest.spec.ts`)

```ts
import { expect } from '@open-wc/testing';

describe('elohim-qahal-foo custom-elements-manifest', () => {
  let declaration: CemDeclaration;
  before(async () => {
    const manifest = await (await fetch('/dist/custom-elements.json')).json();
    declaration = manifest.modules
      .flatMap(m => m.declarations ?? [])
      .find(d => d.tagName === 'elohim-qahal-foo');
  });
  it('declares the tag', () => expect(declaration).to.exist);
  it('declares expected capability JSDoc tags', () => { /* ... */ });
});
```

(Full type definitions are in `app/elohim-elements/elohim-core/src/elohim-button.manifest.spec.ts` — reuse via copy-and-adapt.)

### Default story pattern

```ts
// elohim-qahal-foo.stories.ts
import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';
import '@elohim/elohim-qahal/register';

const meta: Meta = {
  title: 'default/qahal/elohim-qahal-foo',
  tags: ['autodocs'],
};
export default meta;

export const Unstyled: StoryObj = {
  render: () => html`<elohim-qahal-foo>Content</elohim-qahal-foo>`,
};
export const CustomTheme: StoryObj = {
  render: () => html`
    <div style="--elohim-qahal-foo-bg: #d4a373;">
      <elohim-qahal-foo>Themed content</elohim-qahal-foo>
    </div>
  `,
};
```

### Commit messages

Each task ends with a commit using conventional commits:
- `feat(elohim-qahal): add elohim-qahal-imagodei-badge primitive`
- `feat(elohim-imagodei): add settings-palette element`
- `test(elohim-qahal): add a11y tests for stream panel`

---

## File structure (full plan scope)

```
app/elohim-elements/elohim-qahal/
  src/
    # primitives (Phase 1)
    elohim-qahal-imagodei-badge.ts        + .spec.ts + .manifest.spec.ts
    elohim-qahal-standing-ring.ts          + .spec.ts + .manifest.spec.ts
    elohim-qahal-capability-tier-chip.ts   + .spec.ts + .manifest.spec.ts
    elohim-qahal-provenance-marker.ts      + .spec.ts + .manifest.spec.ts
    elohim-qahal-care-economy-marker.ts    + .spec.ts + .manifest.spec.ts

    # chrome (Phase 3)
    elohim-qahal-collective-switcher.ts    + .spec.ts + .manifest.spec.ts
    elohim-qahal-sidebar.ts                 + .spec.ts + .manifest.spec.ts
    elohim-qahal-main-viewer.ts             + .spec.ts + .manifest.spec.ts
    elohim-qahal-context-column.ts          + .spec.ts + .manifest.spec.ts

    # deep-impl panels (Phase 4)
    elohim-qahal-stream-panel.ts            + .spec.ts + .manifest.spec.ts
    elohim-qahal-member-ring-panel.ts       + .spec.ts + .manifest.spec.ts
    elohim-qahal-rules-panel.ts             + .spec.ts + .manifest.spec.ts
    elohim-qahal-co-steward-panel.ts        + .spec.ts + .manifest.spec.ts
    elohim-qahal-social-compute-panel.ts    + .spec.ts + .manifest.spec.ts

    # visual-stub panels (Phase 5)
    elohim-qahal-standing-inspector-panel.ts  + .spec.ts + .manifest.spec.ts
    elohim-qahal-shefa-resources-panel.ts     + .spec.ts + .manifest.spec.ts
    elohim-qahal-attestations-panel.ts        + .spec.ts + .manifest.spec.ts
    elohim-qahal-graph-discovery-panel.ts     + .spec.ts + .manifest.spec.ts

    # resource list (Phase 6)
    elohim-qahal-protocol-panel-list.ts     + .spec.ts + .manifest.spec.ts
    elohim-qahal-curated-epr-list.ts        + .spec.ts + .manifest.spec.ts
    elohim-qahal-external-link-list.ts      + .spec.ts + .manifest.spec.ts
    elohim-qahal-power-user-expandable.ts   + .spec.ts + .manifest.spec.ts

    register.ts
    index.ts
  package.json (extended from styles-only to Lit element library)
  vite.config.ts
  custom-elements-manifest.config.mjs

app/elohim-elements/elohim-imagodei/
  src/
    # settings palette + introspection (Phase 7)
    elohim-imagodei-settings-palette.ts          + .spec.ts + .manifest.spec.ts
    elohim-imagodei-setting-control.ts           + .spec.ts + .manifest.spec.ts
    elohim-imagodei-protected-tier-marker.ts     + .spec.ts + .manifest.spec.ts
    elohim-imagodei-steward-configure-banner.ts  + .spec.ts + .manifest.spec.ts
    elohim-imagodei-introspection-panel.ts       + .spec.ts + .manifest.spec.ts
    register.ts
    index.ts
  package.json + vite.config.ts + cem config

app/elohim-library/projects/graphos/src/default/qahal/
  fixtures/  (Phase 2 — mock-data primitives)
    mock-imagodei-profiles.ts
    mock-rubrics.ts
    mock-care-economy-events.ts
    mock-social-compute-topology.ts
```

**Total element count:** 23 qahal elements + 5 imagodei elements = 28 elements. Plus 4 mock-data fixture modules. ~32 tasks across 8 phases.

---

## Phase 0 — Workspace scaffolding

### Task 0.1 — Initialize elohim-qahal and elohim-imagodei packages as Lit element libraries

**Files:**
- Modify: `app/elohim-elements/elohim-qahal/package.json` (extend styles-only to Lit element library)
- Modify: `app/elohim-elements/elohim-imagodei/package.json` (same)
- Create: `app/elohim-elements/elohim-qahal/vite.config.ts`
- Create: `app/elohim-elements/elohim-qahal/custom-elements-manifest.config.mjs`
- Create: `app/elohim-elements/elohim-qahal/tsconfig.json`
- Create: `app/elohim-elements/elohim-qahal/src/register.ts`
- Create: `app/elohim-elements/elohim-qahal/src/index.ts`
- Create: equivalent files for `elohim-imagodei` package
- Reference: `app/elohim-elements/elohim-core/` is the canonical example to copy

- [ ] **Step 1: Read elohim-core package configuration**

Run: `cat app/elohim-elements/elohim-core/package.json app/elohim-elements/elohim-core/vite.config.ts app/elohim-elements/elohim-core/custom-elements-manifest.config.mjs app/elohim-elements/elohim-core/tsconfig.json`
Expected: Shows the canonical config to mirror.

- [ ] **Step 2: Update `app/elohim-elements/elohim-qahal/package.json`**

Replace the styles-only package.json with the Lit-element-library equivalent (mirrors `elohim-core/package.json`):

```json
{
  "name": "elohim-qahal",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "sideEffects": [
    "./src/register.ts",
    "./dist/register.js",
    "*.css",
    "*.scss"
  ],
  "description": "Community pillar — Qahal homepage elements: chrome, panels, resource list.",
  "style": "./index.scss",
  "sass": "./index.scss",
  "main": "./dist/index.js",
  "module": "./dist/index.js",
  "types": "./dist/index.d.ts",
  "customElements": "./dist/custom-elements.json",
  "exports": {
    ".": { "import": "./dist/index.js", "types": "./dist/index.d.ts" },
    "./register": { "import": "./dist/register.js", "types": "./dist/register.d.ts" },
    "./tokens.scss": "./tokens.scss"
  },
  "files": ["dist", "*.scss", "README.md"],
  "scripts": {
    "analyze": "cem analyze --config custom-elements-manifest.config.mjs",
    "build": "vite build && cem analyze --config custom-elements-manifest.config.mjs",
    "dev": "vite build --watch",
    "test": "web-test-runner --config ../web-test-runner.config.mjs",
    "format": "prettier --config ../.prettierrc.js --ignore-path ../.prettierignore --write \"src/**/*.ts\" \"*.scss\"",
    "lint": "eslint --config ../eslint.config.js src",
    "lint:fix": "eslint --config ../eslint.config.js src --fix"
  },
  "dependencies": {
    "lit": "^3.0.0",
    "elohim-core": "workspace:*"
  },
  "devDependencies": {
    "@custom-elements-manifest/analyzer": "^0.10.0",
    "@open-wc/testing": "^4.0.0",
    "@types/mocha": "^10.0.0",
    "@web/test-runner": "^0.18.0",
    "axe-core": "^4.10.0",
    "typescript": "^5.4.0",
    "vite": "^5.0.0"
  }
}
```

- [ ] **Step 3: Create `vite.config.ts` (mirror elohim-core)**

```ts
import { defineConfig } from 'vite';
import { resolve } from 'node:path';
export default defineConfig({
  build: {
    lib: {
      entry: { index: resolve(__dirname, 'src/index.ts'), register: resolve(__dirname, 'src/register.ts') },
      formats: ['es'],
    },
    rollupOptions: { external: ['lit', /^lit\//, /^@elohim\/elohim-core/], output: { preserveModules: true, dir: 'dist' } },
    sourcemap: true,
  },
});
```

- [ ] **Step 4: Create `custom-elements-manifest.config.mjs`**

```js
export default {
  globs: ['src/**/*.ts'],
  exclude: ['src/**/*.spec.ts', 'src/**/*.manifest.spec.ts', 'src/register.ts', 'src/index.ts'],
  outdir: 'dist',
  litelement: true,
  packagejson: false,
  plugins: [],
};
```

- [ ] **Step 5: Create `tsconfig.json`**

```json
{
  "extends": "../tsconfig.json",
  "compilerOptions": { "outDir": "./dist", "rootDir": "./src", "composite": true },
  "include": ["src/**/*.ts"],
  "exclude": ["src/**/*.spec.ts", "src/**/*.manifest.spec.ts"]
}
```

- [ ] **Step 6: Create `src/register.ts` (initially empty; each task appends its element)**

```ts
// elohim-qahal — element registration entry point.
// Each element registers its tag here so consumers can `import 'elohim-qahal/register'`
// to make all custom elements available.
```

- [ ] **Step 7: Create `src/index.ts` (initially empty; each task appends its exports)**

```ts
// elohim-qahal — public API surface for type imports.
// Consumers import the element classes for type annotations:
// `import type { ElohimQahalFoo } from 'elohim-qahal';`
```

- [ ] **Step 8: Repeat steps 2-7 for `app/elohim-elements/elohim-imagodei/`**

Same shape, substitute `elohim-imagodei` for `elohim-qahal` and adjust the description to `"Identity pillar — settings palette + introspection elements."`

- [ ] **Step 9: Verify pnpm workspace install**

Run: `pnpm install --filter elohim-qahal --filter elohim-imagodei`
Expected: All deps install; no errors.

- [ ] **Step 10: Verify build of empty packages succeeds**

Run: `pnpm --filter elohim-qahal run build && pnpm --filter elohim-imagodei run build`
Expected: Both `dist/` directories created; `dist/custom-elements.json` exists for each.

- [ ] **Step 11: Commit**

```bash
git add app/elohim-elements/elohim-qahal app/elohim-elements/elohim-imagodei
git commit -m "chore(elohim-elements): scaffold elohim-qahal + elohim-imagodei as Lit element libraries"
```

---

## Phase 1 — Qahal primitives (5 elements)

These primitives are used by every panel and the chrome. Build them first so downstream tasks can depend on them.

### Task 1.1 — elohim-qahal-imagodei-badge primitive

**Purpose:** Small inline badge rendering a human's imagodei (avatar + name + their standing ring in this Qahal's lens). Used in stream items, member-ring drill-downs, and co-steward observations. Substrate primitive for "who is this person in this Qahal."

**Files:**
- Create: `app/elohim-elements/elohim-qahal/src/elohim-qahal-imagodei-badge.ts`
- Test: `app/elohim-elements/elohim-qahal/src/elohim-qahal-imagodei-badge.spec.ts`
- Test: `app/elohim-elements/elohim-qahal/src/elohim-qahal-imagodei-badge.manifest.spec.ts`
- Modify: `app/elohim-elements/elohim-qahal/src/register.ts` (append element registration)
- Modify: `app/elohim-elements/elohim-qahal/src/index.ts` (append type export)

- [ ] **Step 1: Write the failing element test**

```ts
// elohim-qahal-imagodei-badge.spec.ts
import { expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import './register.js';
import type { ElohimQahalImagodeiBadge } from './elohim-qahal-imagodei-badge.js';

describe('<elohim-qahal-imagodei-badge>', () => {
  it('renders with display name in default slot', async () => {
    const el = await fixture<ElohimQahalImagodeiBadge>(html`
      <elohim-qahal-imagodei-badge name="Matthew Dowell"></elohim-qahal-imagodei-badge>
    `);
    expect(el.shadowRoot).to.exist;
    expect(el.shadowRoot!.textContent).to.include('Matthew Dowell');
  });

  it('exposes name and avatar-url properties', async () => {
    const el = await fixture<ElohimQahalImagodeiBadge>(html`
      <elohim-qahal-imagodei-badge
        name="Matthew Dowell"
        avatar-url="https://example.com/m.jpg"
      ></elohim-qahal-imagodei-badge>
    `);
    expect(el.name).to.equal('Matthew Dowell');
    expect(el.avatarUrl).to.equal('https://example.com/m.jpg');
  });

  it('supports a standing-tier attribute (visitor | engaged | contributor | steward)', async () => {
    const el = await fixture<ElohimQahalImagodeiBadge>(html`
      <elohim-qahal-imagodei-badge name="X" standing-tier="steward"></elohim-qahal-imagodei-badge>
    `);
    expect(el.standingTier).to.equal('steward');
    expect(el.getAttribute('standing-tier')).to.equal('steward');
  });

  it('passes axe accessibility audit', async () => {
    const el = await fixture<ElohimQahalImagodeiBadge>(html`
      <elohim-qahal-imagodei-badge name="Matthew"></elohim-qahal-imagodei-badge>
    `);
    const results = await axe.run(el);
    expect(results.violations).to.be.empty;
  });

  it('renders fallback initials when avatar-url is absent', async () => {
    const el = await fixture<ElohimQahalImagodeiBadge>(html`
      <elohim-qahal-imagodei-badge name="Matthew Dowell"></elohim-qahal-imagodei-badge>
    `);
    expect(el.shadowRoot!.textContent).to.include('MD');
  });
});
```

- [ ] **Step 2: Run test, verify FAIL**

Run: `pnpm --filter elohim-qahal run test -- --grep imagodei-badge`
Expected: FAIL with "cannot find module './elohim-qahal-imagodei-badge.js'" or similar.

- [ ] **Step 3: Implement the element**

```ts
// elohim-qahal-imagodei-badge.ts
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';
import { CapabilityAwareElement } from '@elohim/elohim-core/capability';

export type StandingTier = 'visitor' | 'engaged' | 'contributor' | 'steward';

/**
 * Inline imagodei badge — avatar + name + standing ring, lensed through current Qahal.
 *
 * @element elohim-qahal-imagodei-badge
 * @prop {string} name - The human's display name
 * @prop {string} avatarUrl - Optional avatar image URL
 * @prop {StandingTier} standingTier - Their standing in this Qahal
 * @cssprop --elohim-qahal-imagodei-badge-size - Override size (default 1.5rem)
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings visitor | engaged | contributor | steward | elohim-support
 * @capabilityContentCertainty observed-private
 * @capabilityStates empty:supported, loading:supported, error:supported, stale:supported, contested:n/a, offline:supported, unauthorized:supported
 */
export class ElohimQahalImagodeiBadge extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host { display: inline-flex; align-items: center; gap: 0.5rem; }
    .avatar { width: var(--elohim-qahal-imagodei-badge-size, 1.5rem); height: var(--elohim-qahal-imagodei-badge-size, 1.5rem); border-radius: 50%; background: var(--elohim-color-surface-2, #eee); display: inline-flex; align-items: center; justify-content: center; font-size: 0.75rem; font-weight: 500; }
    .avatar img { width: 100%; height: 100%; border-radius: 50%; object-fit: cover; }
    .name { font-weight: 500; }
    .ring { width: 0.5rem; height: 0.5rem; border-radius: 50%; }
    .ring[data-tier="visitor"] { background: var(--elohim-tier-visitor, #ccc); }
    .ring[data-tier="engaged"] { background: var(--elohim-tier-engaged, #99c); }
    .ring[data-tier="contributor"] { background: var(--elohim-tier-contributor, #6c9); }
    .ring[data-tier="steward"] { background: var(--elohim-tier-steward, #c96); }
  `;

  @property({ type: String }) name = '';
  @property({ type: String, attribute: 'avatar-url' }) avatarUrl = '';
  @property({ type: String, reflect: true, attribute: 'standing-tier' }) standingTier: StandingTier = 'visitor';

  private get initials(): string {
    return this.name.split(/\s+/).map(w => w[0] ?? '').slice(0, 2).join('').toUpperCase();
  }

  override render() {
    return html`
      <span class="avatar" aria-hidden="true">
        ${this.avatarUrl ? html`<img src=${this.avatarUrl} alt="" />` : html`<span>${this.initials}</span>`}
      </span>
      <span class="name">${this.name}</span>
      <span class="ring" data-tier=${this.standingTier} aria-label="Standing tier: ${this.standingTier}"></span>
    `;
  }
}
```

- [ ] **Step 4: Register the element**

Append to `src/register.ts`:
```ts
import { ElohimQahalImagodeiBadge } from './elohim-qahal-imagodei-badge.js';
if (!customElements.get('elohim-qahal-imagodei-badge')) {
  customElements.define('elohim-qahal-imagodei-badge', ElohimQahalImagodeiBadge);
}
```

Append to `src/index.ts`:
```ts
export { ElohimQahalImagodeiBadge } from './elohim-qahal-imagodei-badge.js';
```

- [ ] **Step 5: Run test, verify PASS**

Run: `pnpm --filter elohim-qahal run test -- --grep imagodei-badge`
Expected: All assertions pass.

- [ ] **Step 6: Write the manifest spec test**

```ts
// elohim-qahal-imagodei-badge.manifest.spec.ts
import { expect } from '@open-wc/testing';
interface CemDecl { name: string; tagName?: string; members?: { kind: string; name: string }[]; jsDoc?: string; }
interface CemManifest { modules: { declarations?: CemDecl[] }[]; }

describe('elohim-qahal-imagodei-badge custom-elements-manifest', () => {
  let decl: CemDecl;
  before(async () => {
    const res = await fetch('/dist/custom-elements.json');
    const manifest = await res.json() as CemManifest;
    decl = manifest.modules.flatMap(m => m.declarations ?? []).find(d => d.tagName === 'elohim-qahal-imagodei-badge')!;
  });
  it('declares the tag', () => expect(decl).to.exist);
  it('declares the name property', () => {
    const prop = decl.members?.find(m => m.kind === 'field' && m.name === 'name');
    expect(prop).to.exist;
  });
  it('declares the avatarUrl property', () => {
    expect(decl.members?.find(m => m.name === 'avatarUrl')).to.exist;
  });
  it('declares the standingTier property', () => {
    expect(decl.members?.find(m => m.name === 'standingTier')).to.exist;
  });
});
```

- [ ] **Step 7: Run build + manifest analyze + manifest test**

Run: `pnpm --filter elohim-qahal run build && pnpm --filter elohim-qahal run test -- --grep imagodei-badge.manifest`
Expected: Both build and tests pass.

- [ ] **Step 8: Commit**

```bash
git add app/elohim-elements/elohim-qahal/src/elohim-qahal-imagodei-badge.ts app/elohim-elements/elohim-qahal/src/elohim-qahal-imagodei-badge.spec.ts app/elohim-elements/elohim-qahal/src/elohim-qahal-imagodei-badge.manifest.spec.ts app/elohim-elements/elohim-qahal/src/register.ts app/elohim-elements/elohim-qahal/src/index.ts
git commit -m "feat(elohim-qahal): add elohim-qahal-imagodei-badge primitive"
```

### Task 1.2 — elohim-qahal-standing-ring primitive

**Purpose:** Small ring indicator showing standing as Bloom-tier dots (◯◯◯ or ●●○ etc.). Visualizes capability tier at a glance. Used in member-ring and standing-inspector panels.

**Files:**
- Create: `app/elohim-elements/elohim-qahal/src/elohim-qahal-standing-ring.ts` + `.spec.ts` + `.manifest.spec.ts`
- Modify: `register.ts`, `index.ts`

- [ ] **Step 1: Write the failing test**

```ts
// elohim-qahal-standing-ring.spec.ts (excerpt — full follows pattern of Task 1.1)
import { expect, fixture, html } from '@open-wc/testing';
import './register.js';
import type { ElohimQahalStandingRing } from './elohim-qahal-standing-ring.js';

describe('<elohim-qahal-standing-ring>', () => {
  it('renders with bloom-tier attribute', async () => {
    const el = await fixture<ElohimQahalStandingRing>(html`
      <elohim-qahal-standing-ring bloom-tier="apply"></elohim-qahal-standing-ring>
    `);
    expect(el.shadowRoot!.textContent).to.match(/●●●○○○/);
  });
  it('supports six bloom tiers', async () => {
    const tiers = ['remember', 'understand', 'apply', 'analyze', 'evaluate', 'create'];
    for (const t of tiers) {
      const el = await fixture<ElohimQahalStandingRing>(html`
        <elohim-qahal-standing-ring bloom-tier=${t}></elohim-qahal-standing-ring>
      `);
      expect(el.bloomTier).to.equal(t);
    }
  });
  it('aria-labels the bloom tier', async () => {
    const el = await fixture<ElohimQahalStandingRing>(html`
      <elohim-qahal-standing-ring bloom-tier="apply"></elohim-qahal-standing-ring>
    `);
    expect(el.shadowRoot!.querySelector('[role="img"]')?.getAttribute('aria-label')).to.include('apply');
  });
});
```

- [ ] **Step 2: Run, verify FAIL**

Run: `pnpm --filter elohim-qahal run test -- --grep standing-ring`

- [ ] **Step 3: Implement**

```ts
// elohim-qahal-standing-ring.ts
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';
import { CapabilityAwareElement } from '@elohim/elohim-core/capability';

export type BloomTier = 'remember' | 'understand' | 'apply' | 'analyze' | 'evaluate' | 'create';

const TIERS: BloomTier[] = ['remember', 'understand', 'apply', 'analyze', 'evaluate', 'create'];

/**
 * Standing ring — Bloom-tier dots indicating capability tier at a glance.
 *
 * @element elohim-qahal-standing-ring
 * @prop {BloomTier} bloomTier - The tier to render (remember | understand | apply | analyze | evaluate | create)
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en
 * @capabilityMaxStimulus still
 * @capabilityTextuality symbolic
 * @capabilityRequiredStandings visitor | engaged | contributor | steward | elohim-support
 * @capabilityContentCertainty observed-private
 * @capabilityStates empty:supported, loading:n/a, error:n/a, stale:supported, contested:n/a, offline:supported, unauthorized:n/a
 */
export class ElohimQahalStandingRing extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host { display: inline-block; }
    [role="img"] { font-family: monospace; letter-spacing: -0.1em; color: var(--elohim-color-fg-2, #555); }
  `;

  @property({ type: String, reflect: true, attribute: 'bloom-tier' }) bloomTier: BloomTier = 'remember';

  override render() {
    const idx = TIERS.indexOf(this.bloomTier) + 1;
    const filled = '●'.repeat(idx);
    const empty = '○'.repeat(6 - idx);
    return html`<span role="img" aria-label="Bloom tier: ${this.bloomTier} (${idx} of 6)">${filled}${empty}</span>`;
  }
}
```

- [ ] **Step 4: Register + export (same pattern as Task 1.1)**

- [ ] **Step 5: Run test, verify PASS**

- [ ] **Step 6: Write manifest spec test (mirror Task 1.1)**

- [ ] **Step 7: Build + manifest test pass**

- [ ] **Step 8: Commit**

```bash
git commit -m "feat(elohim-qahal): add elohim-qahal-standing-ring primitive"
```

### Task 1.3 — elohim-qahal-capability-tier-chip primitive

**Purpose:** Inline chip showing capability tier (visitor / engaged / contributor / steward / protected-tier-name) with appropriate color treatment. Used for capability-gating affordances.

**Files:**
- Create: `app/elohim-elements/elohim-qahal/src/elohim-qahal-capability-tier-chip.ts` + `.spec.ts` + `.manifest.spec.ts`

- [ ] **Step 1: Write failing test**

```ts
describe('<elohim-qahal-capability-tier-chip>', () => {
  it('renders the tier label', async () => {
    const el = await fixture<ElohimQahalCapabilityTierChip>(html`
      <elohim-qahal-capability-tier-chip tier="steward"></elohim-qahal-capability-tier-chip>
    `);
    expect(el.shadowRoot!.textContent).to.include('steward');
  });
  it('supports protected-tier values (child, idd_member, elder_under_guardianship, legal_steward_protected)', async () => {
    for (const t of ['child', 'idd_member', 'elder_under_guardianship', 'legal_steward_protected']) {
      const el = await fixture<ElohimQahalCapabilityTierChip>(html`
        <elohim-qahal-capability-tier-chip tier=${t}></elohim-qahal-capability-tier-chip>
      `);
      expect(el.tier).to.equal(t);
    }
  });
  it('marks protected tiers visually distinctly', async () => {
    const el = await fixture<ElohimQahalCapabilityTierChip>(html`
      <elohim-qahal-capability-tier-chip tier="child"></elohim-qahal-capability-tier-chip>
    `);
    expect(el.getAttribute('protected')).to.not.be.null;
  });
});
```

- [ ] **Step 2: Run, verify FAIL**

- [ ] **Step 3: Implement**

```ts
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';
import { CapabilityAwareElement } from '@elohim/elohim-core/capability';

export type CapabilityTier =
  | 'visitor' | 'engaged' | 'contributor' | 'steward' | 'elohim-support'
  | 'child' | 'idd_member' | 'elder_under_guardianship' | 'legal_steward_protected';

const PROTECTED: CapabilityTier[] = ['child', 'idd_member', 'elder_under_guardianship', 'legal_steward_protected'];

/**
 * Capability tier chip — inline label for a tier with visual distinction for protected tiers.
 *
 * @element elohim-qahal-capability-tier-chip
 * @prop {CapabilityTier} tier - The tier to display
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings visitor | engaged | contributor | steward | elohim-support
 * @capabilityContentCertainty observed-private
 * @capabilityStates empty:n/a, loading:n/a, error:n/a, stale:n/a, contested:n/a, offline:n/a, unauthorized:n/a
 */
export class ElohimQahalCapabilityTierChip extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host { display: inline-block; }
    .chip { display: inline-block; padding: 0.125rem 0.5rem; border-radius: 999px; font-size: 0.75rem; font-weight: 500; background: var(--elohim-color-surface-2, #eee); color: var(--elohim-color-fg-1, #222); }
    :host([protected]) .chip { background: var(--elohim-color-protected-bg, #fef3c7); color: var(--elohim-color-protected-fg, #92400e); border: 1px solid var(--elohim-color-protected-border, #fbbf24); }
  `;

  @property({ type: String, reflect: true }) tier: CapabilityTier = 'visitor';

  override willUpdate(changed: Map<string, unknown>) {
    if (changed.has('tier')) {
      if (PROTECTED.includes(this.tier)) {
        this.setAttribute('protected', '');
      } else {
        this.removeAttribute('protected');
      }
    }
  }

  private get label(): string {
    return this.tier.replace(/_/g, ' ');
  }

  override render() {
    return html`<span class="chip">${this.label}</span>`;
  }
}
```

- [ ] **Step 4-8: Register, export, test, manifest spec, commit (pattern as Task 1.1)**

```bash
git commit -m "feat(elohim-qahal): add elohim-qahal-capability-tier-chip primitive with protected-tier marking"
```

### Task 1.4 — elohim-qahal-provenance-marker primitive

**Purpose:** Marker icon (●/◆/⬢/⤤) for the four provenance categories in the resource list (protocol-panel / curated-EPR / installed-applet / external-hyperlink).

**Files:** `elohim-qahal-provenance-marker.ts` + `.spec.ts` + `.manifest.spec.ts`

- [ ] **Step 1: Write failing test**

```ts
describe('<elohim-qahal-provenance-marker>', () => {
  it('renders the four provenance categories with distinct symbols', async () => {
    const cases: Array<[string, string]> = [
      ['protocol-panel', '●'],
      ['curated-epr', '◆'],
      ['installed-applet', '⬢'],
      ['external-hyperlink', '⤤'],
    ];
    for (const [category, expected] of cases) {
      const el = await fixture(html`
        <elohim-qahal-provenance-marker category=${category}></elohim-qahal-provenance-marker>
      `);
      expect(el.shadowRoot!.textContent).to.include(expected);
    }
  });
  it('marks external-hyperlink as offline-greyable', async () => {
    const el = await fixture<ElohimQahalProvenanceMarker>(html`
      <elohim-qahal-provenance-marker category="external-hyperlink" offline></elohim-qahal-provenance-marker>
    `);
    expect(el.offline).to.be.true;
  });
});
```

- [ ] **Step 2: FAIL**

- [ ] **Step 3: Implement**

```ts
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';
import { CapabilityAwareElement } from '@elohim/elohim-core/capability';

export type ProvenanceCategory = 'protocol-panel' | 'curated-epr' | 'installed-applet' | 'external-hyperlink';

const SYMBOLS: Record<ProvenanceCategory, string> = {
  'protocol-panel': '●',
  'curated-epr': '◆',
  'installed-applet': '⬢',
  'external-hyperlink': '⤤',
};

const LABELS: Record<ProvenanceCategory, string> = {
  'protocol-panel': 'protocol panel',
  'curated-epr': 'curated EPR',
  'installed-applet': 'installed applet',
  'external-hyperlink': 'external hyperlink (leaving the elohim network)',
};

/**
 * Provenance marker — symbol denoting which substrate category an item belongs to.
 *
 * @element elohim-qahal-provenance-marker
 * @prop {ProvenanceCategory} category - The provenance category
 * @prop {boolean} offline - When true, render in greyed-out style (applicable to external-hyperlink)
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en
 * @capabilityMaxStimulus still
 * @capabilityTextuality symbolic
 * @capabilityRequiredStandings visitor | engaged | contributor | steward | elohim-support
 * @capabilityContentCertainty not-observed
 * @capabilityStates empty:n/a, loading:n/a, error:n/a, stale:n/a, contested:n/a, offline:supported, unauthorized:n/a
 */
export class ElohimQahalProvenanceMarker extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host { display: inline-block; font-size: 0.875rem; }
    [data-offline] { opacity: 0.4; filter: grayscale(1); }
  `;

  @property({ type: String, reflect: true }) category: ProvenanceCategory = 'protocol-panel';
  @property({ type: Boolean, reflect: true }) offline = false;

  override render() {
    return html`<span ?data-offline=${this.offline} aria-label=${LABELS[this.category]}>${SYMBOLS[this.category]}</span>`;
  }
}
```

- [ ] **Step 4-8: Register, export, test, manifest, commit**

```bash
git commit -m "feat(elohim-qahal): add elohim-qahal-provenance-marker primitive (●◆⬢⤤)"
```

### Task 1.5 — elohim-qahal-care-economy-marker primitive

**Purpose:** Small marker for REA care-economy events in the stream (+ tokens, presence attestation, repair offered). Used inline in stream items.

**Files:** standard set

- [ ] **Step 1: Write failing test**

```ts
describe('<elohim-qahal-care-economy-marker>', () => {
  it('renders kind="care" with positive token count', async () => {
    const el = await fixture<ElohimQahalCareEconomyMarker>(html`
      <elohim-qahal-care-economy-marker kind="care" tokens="5"></elohim-qahal-care-economy-marker>
    `);
    expect(el.shadowRoot!.textContent).to.match(/\+5.*care/);
  });
  it('supports kinds: care, presence, repair, growth, time', async () => {
    for (const k of ['care', 'presence', 'repair', 'growth', 'time']) {
      const el = await fixture(html`
        <elohim-qahal-care-economy-marker kind=${k} tokens="1"></elohim-qahal-care-economy-marker>
      `);
      expect(el.getAttribute('kind')).to.equal(k);
    }
  });
});
```

- [ ] **Step 2-3: FAIL then implement**

```ts
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';
import { CapabilityAwareElement } from '@elohim/elohim-core/capability';

export type CareEconomyKind = 'care' | 'presence' | 'repair' | 'growth' | 'time';

const ICONS: Record<CareEconomyKind, string> = {
  care: '✨', presence: '👁', repair: '🛠', growth: '🌱', time: '⏰',
};

/**
 * Care-economy marker — small inline REA event indicator.
 *
 * @element elohim-qahal-care-economy-marker
 * @prop {CareEconomyKind} kind - Which kind of care-economy contribution
 * @prop {number} tokens - Count of tokens (default 1)
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings visitor | engaged | contributor | steward | elohim-support
 * @capabilityContentCertainty observed-private
 * @capabilityStates empty:n/a, loading:n/a, error:n/a, stale:supported, contested:n/a, offline:supported, unauthorized:n/a
 */
export class ElohimQahalCareEconomyMarker extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host { display: inline-flex; align-items: center; gap: 0.25rem; font-size: 0.75rem; color: var(--elohim-color-fg-2, #555); }
  `;

  @property({ type: String, reflect: true }) kind: CareEconomyKind = 'care';
  @property({ type: Number }) tokens = 1;

  override render() {
    return html`
      <span aria-label="${this.kind} contribution, ${this.tokens} tokens">
        ${ICONS[this.kind]} +${this.tokens} ${this.kind}
      </span>
    `;
  }
}
```

- [ ] **Step 4-8: Register, export, test, manifest, commit**

```bash
git commit -m "feat(elohim-qahal): add elohim-qahal-care-economy-marker primitive"
```

---

## Phase 2 — Mock-data primitives (4 fixture modules)

These TypeScript modules provide typed mock data for stories and tests across all elements. They live in `app/elohim-library/projects/graphos/src/default/qahal/fixtures/`.

### Task 2.1 — mock-imagodei-profiles

**Files:**
- Create: `app/elohim-library/projects/graphos/src/default/qahal/fixtures/mock-imagodei-profiles.ts`

- [ ] **Step 1: Write the fixture module**

```ts
// mock-imagodei-profiles.ts
export interface MockImagodeiProfile {
  id: string;
  name: string;
  avatarUrl?: string;
  standingTier: 'visitor' | 'engaged' | 'contributor' | 'steward';
  bloomTier: 'remember' | 'understand' | 'apply' | 'analyze' | 'evaluate' | 'create';
  capabilityTier: 'visitor' | 'engaged' | 'contributor' | 'steward' | 'elohim-support' | 'child' | 'idd_member' | 'elder_under_guardianship' | 'legal_steward_protected';
  affiliations: string[];
}

export const DOWELL_FAMILY: MockImagodeiProfile[] = [
  { id: 'matthew-dowell', name: 'Matthew Dowell', standingTier: 'steward', bloomTier: 'create', capabilityTier: 'steward', affiliations: ['dowell-household', 'cofc-congregation'] },
  { id: 'jessica-dowell', name: 'Jessica Dowell', standingTier: 'steward', bloomTier: 'create', capabilityTier: 'steward', affiliations: ['dowell-household', 'cofc-congregation'] },
  { id: 'james-dowell', name: 'James Dowell', standingTier: 'engaged', bloomTier: 'apply', capabilityTier: 'child', affiliations: ['dowell-household'] },
  { id: 'sheila-household', name: 'Sheila Hardin', standingTier: 'contributor', bloomTier: 'evaluate', capabilityTier: 'steward', affiliations: ['hardin-household', 'cofc-congregation'] },
  { id: 'gertrude-grandma', name: 'Gertrude Dowell', standingTier: 'contributor', bloomTier: 'evaluate', capabilityTier: 'elder_under_guardianship', affiliations: ['gertrude-household', 'cofc-congregation'] },
];

export const COFC_ELDERS: MockImagodeiProfile[] = [
  { id: 'brother-cal', name: 'Brother Cal', standingTier: 'steward', bloomTier: 'create', capabilityTier: 'steward', affiliations: ['cofc-congregation', 'wisdom-commons'] },
  { id: 'elder-thompson', name: 'Elder Thompson', standingTier: 'steward', bloomTier: 'create', capabilityTier: 'steward', affiliations: ['cofc-congregation', 'wisdom-commons'] },
  { id: 'elder-davis', name: 'Elder Davis', standingTier: 'steward', bloomTier: 'create', capabilityTier: 'steward', affiliations: ['cofc-congregation', 'wisdom-commons'] },
  { id: 'elder-rhodes', name: 'Elder Rhodes', standingTier: 'steward', bloomTier: 'evaluate', capabilityTier: 'steward', affiliations: ['cofc-congregation', 'wisdom-commons'] },
];

export const LIFE_GROUP_FAMILIES: MockImagodeiProfile[] = [
  { id: 'john-hardin', name: 'John Hardin', standingTier: 'contributor', bloomTier: 'evaluate', capabilityTier: 'steward', affiliations: ['hardin-household', 'cofc-congregation', 'tuesday-life-group'] },
  { id: 'lee-family', name: 'Jin Lee', standingTier: 'contributor', bloomTier: 'analyze', capabilityTier: 'contributor', affiliations: ['lee-household', 'cofc-congregation', 'tuesday-life-group'] },
  // ... + Robertsons, Kim family
];

export function profileById(id: string): MockImagodeiProfile | undefined {
  return [...DOWELL_FAMILY, ...COFC_ELDERS, ...LIFE_GROUP_FAMILIES].find(p => p.id === id);
}
```

- [ ] **Step 2: Commit**

```bash
git add app/elohim-library/projects/graphos/src/default/qahal/fixtures/mock-imagodei-profiles.ts
git commit -m "feat(graphos): add mock-imagodei-profiles fixture (Dowell + CofC + life-group)"
```

### Task 2.2 — mock-rubrics

**Files:**
- Create: `app/elohim-library/projects/graphos/src/default/qahal/fixtures/mock-rubrics.ts`

- [ ] **Step 1: Write the fixture module**

```ts
export interface MockRubric {
  qahalId: string;
  name: string;
  standingHonors: string[];
  bloomMapping: Record<'remember' | 'understand' | 'apply' | 'analyze' | 'evaluate' | 'create', string>;
  cadenceLabel: string;
  frictionGradientNote: string;
  configuredBy: string[];
  lastRevised: string;
  externalLinkVisibility: Record<string, 'full' | 'filtered_via_co_steward' | 'hidden'>;
}

export const DOWELL_HOUSEHOLD_RUBRIC: MockRubric = {
  qahalId: 'dowell-household',
  name: 'Dowell Household — what we honor here',
  standingHonors: ['care contributed', 'presence shown up', 'repair offered when something has broken between us'],
  bloomMapping: {
    remember: 'know who lives here, what we hold, our rhythms',
    understand: 'explain our care-economy patterns; recognize when help is needed',
    apply: 'do the daily work — meals, chores, attention given without prompt',
    analyze: 'notice when something\'s off; see what\'s not being said',
    evaluate: 'judge contributions against what the household actually needs',
    create: 'propose new rhythms; design our family\'s rule of life',
  },
  cadenceLabel: 'gentle. Standing decays slowly. Old contributions are honored.',
  frictionGradientNote: 'no household member can accumulate disproportionate authority without commons-elohim flagging for discussion. Plural stewardship is structural.',
  configuredBy: ['matthew-dowell', 'jessica-dowell'],
  lastRevised: '2026-04-15',
  externalLinkVisibility: {
    visitor: 'filtered_via_co_steward',
    engaged: 'full',
    contributor: 'full',
    steward: 'full',
    child: 'hidden',
    idd_member: 'filtered_via_co_steward',
    elder_under_guardianship: 'filtered_via_co_steward',
    legal_steward_protected: 'hidden',
  },
};

export const COFC_CONGREGATION_RUBRIC: MockRubric = { /* ... similar shape for congregation ... */ } as MockRubric;
export const LIFE_GROUP_RUBRIC: MockRubric = { /* inherits from congregation ... */ } as MockRubric;
export const WISDOM_COMMONS_RUBRIC: MockRubric = { /* federation template ... */ } as MockRubric;

export function rubricByQahalId(id: string): MockRubric | undefined {
  return [DOWELL_HOUSEHOLD_RUBRIC, COFC_CONGREGATION_RUBRIC, LIFE_GROUP_RUBRIC, WISDOM_COMMONS_RUBRIC].find(r => r.qahalId === id);
}
```

(The "..." in the COFC, life-group, and wisdom-commons rubrics is a placeholder pattern; fill in fully with shape per spec Sections 4.2-4.4. Use the storyteller's canonical narratives as the source — rubric content reflects the congregation's plural-elder stewardship, the life-group's inheritance from congregation, the federation's peer-not-hierarchy stance.)

- [ ] **Step 2: Commit**

```bash
git commit -m "feat(graphos): add mock-rubrics fixture for Tier-0 archetypes"
```

### Task 2.3 — mock-care-economy-events

**Files:**
- Create: `app/elohim-library/projects/graphos/src/default/qahal/fixtures/mock-care-economy-events.ts`

- [ ] **Step 1: Write the fixture module**

```ts
export interface MockCareEconomyEvent {
  id: string;
  qahalId: string;
  authorId: string;
  timestamp: string;
  content: string;
  rea?: { kind: 'care' | 'presence' | 'repair' | 'growth' | 'time'; tokens: number };
  acknowledgmentPending?: boolean;
  threadParentId?: string;
}

export const DOWELL_TUESDAY_MORNING_STREAM: MockCareEconomyEvent[] = [
  { id: 'e1', qahalId: 'dowell-household', authorId: 'jessica-dowell', timestamp: '2026-05-22T07:15:00', content: 'James had a rough night — sick. Taking the morning shift.', rea: { kind: 'care', tokens: 5 } },
  { id: 'e2', qahalId: 'dowell-household', authorId: 'sheila-household', timestamp: '2026-05-22T08:30:00', content: 'Made chicken-and-rice soup last night. Dropping off when I take Connor to school.', rea: { kind: 'care', tokens: 8 }, acknowledgmentPending: true },
  { id: 'e3', qahalId: 'dowell-household', authorId: 'commons-elohim', timestamp: '2026-05-22T09:00:00', content: 'Gertrude checked in. She is glad James is being read to.', acknowledgmentPending: true },
  { id: 'e4', qahalId: 'dowell-household', authorId: 'commons-elohim', timestamp: '2026-05-22T09:30:00', content: 'Neighbor offered to bring dinner. Reply when you have a moment.', acknowledgmentPending: true },
];

export const COFC_SUNDAY_MORNING_STREAM: MockCareEconomyEvent[] = [ /* prayer requests + youth retreat + Romans 12 anchor */ ];
export const LIFE_GROUP_TUESDAY_EVENING_STREAM: MockCareEconomyEvent[] = [ /* Sarah's father + Romans 12 prayer attestations */ ];
export const WISDOM_COMMONS_THURSDAY_STREAM: MockCareEconomyEvent[] = [ /* concern surface + peer council convening */ ];

export function streamForQahal(qahalId: string): MockCareEconomyEvent[] {
  switch (qahalId) {
    case 'dowell-household': return DOWELL_TUESDAY_MORNING_STREAM;
    case 'cofc-congregation': return COFC_SUNDAY_MORNING_STREAM;
    case 'tuesday-life-group': return LIFE_GROUP_TUESDAY_EVENING_STREAM;
    case 'wisdom-commons': return WISDOM_COMMONS_THURSDAY_STREAM;
    default: return [];
  }
}
```

- [ ] **Step 2: Commit**

```bash
git commit -m "feat(graphos): add mock-care-economy-events fixture (4 canonical streams)"
```

### Task 2.4 — mock-social-compute-topology

**Files:**
- Create: `app/elohim-library/projects/graphos/src/default/qahal/fixtures/mock-social-compute-topology.ts`

- [ ] **Step 1: Write the fixture module**

```ts
export interface MockComputeHubStatus {
  hubId: string;
  status: 'healthy' | 'degraded' | 'down';
  allocatedGB: number;
  lastSeen: string;
}

export interface MockComputeStewardRelationship {
  stewardId: string;
  stewardName: string;
  health: 'healthy' | 'degraded' | 'down';
  percent: number;
  lastSync: string;
  replicating: string;
}

export interface MockComputeTopology {
  qahalId: string;
  selfHub: MockComputeHubStatus;
  stewardsForUs: MockComputeStewardRelationship[];
  weStewardFor: { stewardId: string; description: string }[];
  recoveryReadiness: { ready: boolean; shamirThreshold: string; lastDrill: string; nextDrill: string };
}

export const DOWELL_HOUSEHOLD_TOPOLOGY: MockComputeTopology = {
  qahalId: 'dowell-household',
  selfHub: { hubId: 'dowell-hub', status: 'healthy', allocatedGB: 4, lastSeen: 'now' },
  stewardsForUs: [
    { stewardId: 'gertrude-grandma', stewardName: 'gertrude-grandma', health: 'healthy', percent: 100, lastSync: '17m ago', replicating: 'household state + care-economy ledger' },
    { stewardId: 'sheila-household', stewardName: 'sheila-household', health: 'healthy', percent: 100, lastSync: '3m ago', replicating: 'household state' },
    { stewardId: 'ethan-dowell', stewardName: 'ethan-dowell (uncle)', health: 'degraded', percent: 85, lastSync: '4h ago', replicating: 'household state · degraded' },
  ],
  weStewardFor: [
    { stewardId: 'gertrude-grandma', description: 'her household state' },
    { stewardId: 'susan-household', description: 'sibling-household trust' },
  ],
  recoveryReadiness: { ready: true, shamirThreshold: '2/3', lastDrill: '14 days ago (passed)', nextDrill: '16 days' },
};

// Define for cofc-congregation, life-group, wisdom-commons similarly.

export function topologyForQahal(qahalId: string): MockComputeTopology | undefined {
  /* lookup by qahalId */
  if (qahalId === 'dowell-household') return DOWELL_HOUSEHOLD_TOPOLOGY;
  return undefined;
}
```

- [ ] **Step 2: Commit**

```bash
git commit -m "feat(graphos): add mock-social-compute-topology fixture (Dowell topology canonical)"
```

---

## Phase 3 — Chrome (4 elements)

The 4-column layout primitives. These define the page-level slots; panels mount inside them. Each chrome element is **layout-only** — no business logic, just slots + ARIA structure.

### Task 3.1 — elohim-qahal-collective-switcher (far-left column)

**Files:**
- Create: `elohim-qahal-collective-switcher.ts` + `.spec.ts` + `.manifest.spec.ts`

- [ ] **Step 1: Write failing test**

```ts
describe('<elohim-qahal-collective-switcher>', () => {
  it('renders icon list for collectives', async () => {
    const el = await fixture<ElohimQahalCollectiveSwitcher>(html`
      <elohim-qahal-collective-switcher .collectives=${[
        { id: 'dowell-household', icon: '🏠', name: 'Dowell Household' },
        { id: 'cofc-congregation', icon: '⛪', name: 'Congregation' },
      ]} active-collective-id="dowell-household"></elohim-qahal-collective-switcher>
    `);
    const buttons = el.shadowRoot!.querySelectorAll('button');
    expect(buttons.length).to.be.at.least(2);
  });

  it('emits collective-changed event when icon clicked', async () => {
    let emittedId = '';
    const el = await fixture<ElohimQahalCollectiveSwitcher>(html`
      <elohim-qahal-collective-switcher
        .collectives=${[{ id: 'a', icon: '🏠', name: 'A' }, { id: 'b', icon: '⛪', name: 'B' }]}
        @collective-changed=${(e: CustomEvent) => emittedId = e.detail.id}
      ></elohim-qahal-collective-switcher>
    `);
    (el.shadowRoot!.querySelectorAll('button')[1] as HTMLButtonElement).click();
    expect(emittedId).to.equal('b');
  });

  it('marks the active collective visually', async () => {
    const el = await fixture<ElohimQahalCollectiveSwitcher>(html`
      <elohim-qahal-collective-switcher
        .collectives=${[{ id: 'a', icon: '🏠', name: 'A' }]}
        active-collective-id="a"
      ></elohim-qahal-collective-switcher>
    `);
    expect(el.shadowRoot!.querySelector('button[aria-pressed="true"]')).to.exist;
  });
});
```

- [ ] **Step 2: FAIL**

- [ ] **Step 3: Implement**

```ts
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';
import { CapabilityAwareElement } from '@elohim/elohim-core/capability';

export interface CollectiveDescriptor { id: string; icon: string; name: string; }

/**
 * Collective switcher — far-left column listing Qahals the operator participates in.
 *
 * @element elohim-qahal-collective-switcher
 * @prop {CollectiveDescriptor[]} collectives - Array of Qahal descriptors
 * @prop {string} activeCollectiveId - The currently-active collective's id
 * @fires {CustomEvent<{id: string}>} collective-changed - When user clicks a different collective
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings visitor | engaged | contributor | steward | elohim-support
 * @capabilityContentCertainty observed-private
 * @capabilityStates empty:supported, loading:supported, error:supported, stale:n/a, contested:n/a, offline:supported, unauthorized:n/a
 */
export class ElohimQahalCollectiveSwitcher extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host { display: block; padding: 0.5rem 0; background: var(--elohim-color-surface-1, #f7f7f5); border-right: 1px solid var(--elohim-color-border, #ddd); height: 100%; }
    nav { display: flex; flex-direction: column; gap: 0.5rem; }
    button { width: 3rem; height: 3rem; border: 0; background: transparent; cursor: pointer; font-size: 1.5rem; border-radius: 0.5rem; display: flex; align-items: center; justify-content: center; }
    button[aria-pressed="true"] { background: var(--elohim-color-surface-2, #ebebe8); }
    button:focus-visible { outline: 2px solid var(--elohim-color-focus, #6c9); outline-offset: 2px; }
  `;

  @property({ type: Array }) collectives: CollectiveDescriptor[] = [];
  @property({ type: String, attribute: 'active-collective-id' }) activeCollectiveId = '';

  private handleClick(id: string) {
    this.dispatchEvent(new CustomEvent('collective-changed', { detail: { id }, bubbles: true, composed: true }));
  }

  override render() {
    return html`
      <nav aria-label="Collective switcher">
        ${this.collectives.map(c => html`
          <button
            aria-label=${c.name}
            aria-pressed=${c.id === this.activeCollectiveId}
            @click=${() => this.handleClick(c.id)}
          >${c.icon}</button>
        `)}
      </nav>
    `;
  }
}
```

- [ ] **Step 4-8: Register, export, manifest spec, test, commit**

```bash
git commit -m "feat(elohim-qahal): add elohim-qahal-collective-switcher chrome element"
```

### Task 3.2 — elohim-qahal-sidebar

**Purpose:** Second column — the per-Qahal sidebar that holds the four resource-list sections (protocol panels / curated EPRs / external links / power-user expandables). Pure layout container with named slots; actual section elements (from Phase 6) mount into the slots.

**Files:** standard set

- [ ] **Step 1: Write failing test**

```ts
describe('<elohim-qahal-sidebar>', () => {
  it('renders four named slots: panels, curated, external, power-user', async () => {
    const el = await fixture<ElohimQahalSidebar>(html`<elohim-qahal-sidebar></elohim-qahal-sidebar>`);
    const slots = el.shadowRoot!.querySelectorAll('slot');
    const names = Array.from(slots).map(s => s.getAttribute('name')).filter(Boolean);
    expect(names).to.include('panels');
    expect(names).to.include('curated');
    expect(names).to.include('external');
    expect(names).to.include('power-user');
  });
  it('exposes a qahal-name property displayed at top of sidebar', async () => {
    const el = await fixture<ElohimQahalSidebar>(html`
      <elohim-qahal-sidebar qahal-name="Dowell Household"></elohim-qahal-sidebar>
    `);
    expect(el.shadowRoot!.textContent).to.include('Dowell Household');
  });
});
```

- [ ] **Step 2-3: FAIL then implement**

```ts
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';
import { CapabilityAwareElement } from '@elohim/elohim-core/capability';

/**
 * Qahal sidebar — second column of the chrome. Layout container for resource list sections.
 *
 * @element elohim-qahal-sidebar
 * @prop {string} qahalName - Display name of the active Qahal
 * @slot panels - The protocol panels list section
 * @slot curated - The curated EPRs section
 * @slot external - The external hyperlinks section
 * @slot power-user - The power-user expandables section (rendered only if imagodei settings palette has power-user view enabled)
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual
 * @capabilityRequiredStandings visitor | engaged | contributor | steward | elohim-support
 * @capabilityContentCertainty not-observed
 * @capabilityStates empty:supported, loading:supported, error:supported, stale:n/a, contested:n/a, offline:supported, unauthorized:n/a
 */
export class ElohimQahalSidebar extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host { display: block; padding: 1rem; background: var(--elohim-color-surface-1, #fafafa); border-right: 1px solid var(--elohim-color-border, #ddd); height: 100%; overflow-y: auto; }
    h2 { font-size: 0.95rem; font-weight: 600; margin: 0 0 0.75rem 0; }
    section { margin-bottom: 1rem; }
    .section-divider { border: 0; border-top: 1px solid var(--elohim-color-border, #ddd); margin: 0.75rem 0; }
  `;

  @property({ type: String, attribute: 'qahal-name' }) qahalName = '';

  override render() {
    return html`
      <h2>${this.qahalName}</h2>
      <section><slot name="panels"></slot></section>
      <hr class="section-divider" />
      <section><slot name="curated"></slot></section>
      <hr class="section-divider" />
      <section><slot name="external"></slot></section>
      <hr class="section-divider" />
      <section><slot name="power-user"></slot></section>
    `;
  }
}
```

- [ ] **Step 4-8: Register, export, manifest, test, commit**

```bash
git commit -m "feat(elohim-qahal): add elohim-qahal-sidebar chrome element with 4 named slots"
```

### Task 3.3 — elohim-qahal-main-viewer

**Purpose:** Third column — main content area. Renders whichever panel is currently active. Pure layout container with default slot.

**Files:** standard set

- [ ] **Step 1: Write failing test**

```ts
describe('<elohim-qahal-main-viewer>', () => {
  it('renders default slot content', async () => {
    const el = await fixture<ElohimQahalMainViewer>(html`
      <elohim-qahal-main-viewer><div data-testid="content">hello</div></elohim-qahal-main-viewer>
    `);
    const slot = el.shadowRoot!.querySelector('slot');
    expect(slot!.assignedNodes({ flatten: true })).to.have.length.greaterThan(0);
  });
  it('exposes an active-panel-name attribute', async () => {
    const el = await fixture<ElohimQahalMainViewer>(html`
      <elohim-qahal-main-viewer active-panel-name="stream"></elohim-qahal-main-viewer>
    `);
    expect(el.activePanelName).to.equal('stream');
  });
});
```

- [ ] **Step 2-3: FAIL then implement**

```ts
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';
import { CapabilityAwareElement } from '@elohim/elohim-core/capability';

/**
 * Qahal main viewer — third column. Holds the active panel's rendered content.
 *
 * @element elohim-qahal-main-viewer
 * @prop {string} activePanelName - Name of the currently-active panel (for ARIA labelling)
 * @slot - The active panel's element
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual
 * @capabilityRequiredStandings visitor | engaged | contributor | steward | elohim-support
 * @capabilityContentCertainty observed-private
 * @capabilityStates empty:supported, loading:supported, error:supported, stale:supported, contested:supported, offline:supported, unauthorized:supported
 */
export class ElohimQahalMainViewer extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host { display: block; padding: 1.5rem; background: var(--elohim-color-surface-0, #fff); height: 100%; overflow-y: auto; }
  `;

  @property({ type: String, attribute: 'active-panel-name' }) activePanelName = '';

  override render() {
    return html`
      <main role="main" aria-label=${this.activePanelName || 'Active panel'}>
        <slot></slot>
      </main>
    `;
  }
}
```

- [ ] **Step 4-8: Register, export, manifest, test, commit**

```bash
git commit -m "feat(elohim-qahal): add elohim-qahal-main-viewer chrome element"
```

### Task 3.4 — elohim-qahal-context-column

**Purpose:** Fourth column — the right-context column holding co-steward + condensed rules + discovery. Layout container with named slots.

**Files:** standard set

- [ ] **Step 1-3: Test FAIL, implement**

```ts
import { css, html, LitElement } from 'lit';
import { CapabilityAwareElement } from '@elohim/elohim-core/capability';

/**
 * Qahal context column — fourth column. Right-context panels.
 *
 * @element elohim-qahal-context-column
 * @slot co-steward - The commons-elohim co-steward view (always present)
 * @slot rules - Condensed rules summary (collapsible)
 * @slot discovery - Graph discovery suggestions (collapsible)
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings visitor | engaged | contributor | steward | elohim-support
 * @capabilityContentCertainty observed-private
 * @capabilityStates empty:supported, loading:supported, error:supported, stale:supported, contested:n/a, offline:supported, unauthorized:n/a
 */
export class ElohimQahalContextColumn extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host { display: block; padding: 1rem; background: var(--elohim-color-surface-1, #fafafa); border-left: 1px solid var(--elohim-color-border, #ddd); height: 100%; overflow-y: auto; }
    section { margin-bottom: 1.25rem; }
    h3 { font-size: 0.85rem; font-weight: 600; margin: 0 0 0.5rem 0; color: var(--elohim-color-fg-2, #666); text-transform: uppercase; letter-spacing: 0.04em; }
  `;

  override render() {
    return html`
      <aside aria-label="Context">
        <section><h3>Co-steward</h3><slot name="co-steward"></slot></section>
        <section><h3>Rules</h3><slot name="rules"></slot></section>
        <section><h3>Discovery</h3><slot name="discovery"></slot></section>
      </aside>
    `;
  }
}
```

- [ ] **Step 4-8: Register, export, manifest, test, commit**

```bash
git commit -m "feat(elohim-qahal): add elohim-qahal-context-column chrome element"
```

---

## Phase 4 — Deep-implementation panels (5 elements)

These are the panels MVP demos with full visual content. They consume mock-data primitives from Phase 2.

### Task 4.1 — elohim-qahal-stream-panel

**Purpose:** Renders the commons stream — care-economy events + co-steward observations + acknowledgment-pending items. Reads from mock-care-economy-events fixture.

**Files:** standard set

- [ ] **Step 1: Write failing test**

```ts
describe('<elohim-qahal-stream-panel>', () => {
  it('renders stream items with imagodei + timestamp + content', async () => {
    const events = [{ id: 'e1', authorId: 'matthew-dowell', timestamp: '2026-05-22T07:15:00', content: 'James is sick' }];
    const el = await fixture<ElohimQahalStreamPanel>(html`
      <elohim-qahal-stream-panel .events=${events}></elohim-qahal-stream-panel>
    `);
    expect(el.shadowRoot!.textContent).to.include('James is sick');
  });
  it('shows acknowledgment-pending marker for pending items', async () => {
    const events = [{ id: 'e2', authorId: 'sheila-household', timestamp: 't', content: 'Soup arriving', acknowledgmentPending: true }];
    const el = await fixture<ElohimQahalStreamPanel>(html`
      <elohim-qahal-stream-panel .events=${events}></elohim-qahal-stream-panel>
    `);
    expect(el.shadowRoot!.querySelector('[data-acknowledgment-pending]')).to.exist;
  });
  it('renders care-economy markers inline', async () => {
    const events = [{ id: 'e3', authorId: 'm', timestamp: 't', content: 'made breakfast', rea: { kind: 'care', tokens: 5 } }];
    const el = await fixture<ElohimQahalStreamPanel>(html`
      <elohim-qahal-stream-panel .events=${events}></elohim-qahal-stream-panel>
    `);
    expect(el.shadowRoot!.querySelector('elohim-qahal-care-economy-marker')).to.exist;
  });
});
```

- [ ] **Step 2-3: FAIL then implement**

```ts
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';
import { CapabilityAwareElement } from '@elohim/elohim-core/capability';
import './elohim-qahal-imagodei-badge.js';
import './elohim-qahal-care-economy-marker.js';

interface StreamEvent {
  id: string;
  authorId: string;
  timestamp: string;
  content: string;
  rea?: { kind: 'care' | 'presence' | 'repair' | 'growth' | 'time'; tokens: number };
  acknowledgmentPending?: boolean;
}

/**
 * Stream panel — renders the commons stream as a feed.
 *
 * @element elohim-qahal-stream-panel
 * @prop {StreamEvent[]} events - Array of stream events to render
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings visitor | engaged | contributor | steward | elohim-support
 * @capabilityContentCertainty observed-private
 * @capabilityStates empty:supported, loading:supported, error:supported, stale:supported, contested:n/a, offline:supported, unauthorized:supported
 */
export class ElohimQahalStreamPanel extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host { display: block; }
    h2 { font-size: 1.25rem; font-weight: 600; margin: 0 0 1rem 0; }
    .empty { color: var(--elohim-color-fg-2, #666); padding: 2rem 0; text-align: center; }
    ul { list-style: none; padding: 0; margin: 0; }
    li { padding: 0.875rem 0; border-bottom: 1px solid var(--elohim-color-border, #eee); display: flex; gap: 0.75rem; }
    li[data-acknowledgment-pending] { background: var(--elohim-color-pending-bg, #fffbeb); padding: 0.875rem 1rem; border-radius: 0.375rem; border: 1px dashed var(--elohim-color-pending-border, #fbbf24); margin: 0.25rem 0; }
    .meta { display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.25rem; font-size: 0.8rem; color: var(--elohim-color-fg-2, #666); }
    .content { font-size: 0.95rem; }
    .body { flex: 1; }
  `;

  @property({ type: Array }) events: StreamEvent[] = [];

  override render() {
    if (this.events.length === 0) {
      return html`<h2>Stream</h2><div class="empty">No activity yet — the household is quiet.</div>`;
    }
    return html`
      <h2>Stream</h2>
      <ul>
        ${this.events.map(e => html`
          <li ?data-acknowledgment-pending=${e.acknowledgmentPending}>
            <div class="body">
              <div class="meta">
                <elohim-qahal-imagodei-badge name=${e.authorId}></elohim-qahal-imagodei-badge>
                <span>${e.timestamp}</span>
              </div>
              <div class="content">${e.content}</div>
              ${e.rea ? html`<elohim-qahal-care-economy-marker kind=${e.rea.kind} tokens=${e.rea.tokens}></elohim-qahal-care-economy-marker>` : ''}
            </div>
          </li>
        `)}
      </ul>
    `;
  }
}
```

- [ ] **Step 4-8: Register, export, manifest, test, commit**

```bash
git commit -m "feat(elohim-qahal): add elohim-qahal-stream-panel (deep-impl)"
```

### Task 4.2 — elohim-qahal-member-ring-panel (tier-aware)

**Purpose:** Renders the stratified member view — network reach headline + 4 tier sections, each with imagodei drill-downs.

**Files:** standard set

- [ ] **Step 1: Failing test (excerpt)**

```ts
describe('<elohim-qahal-member-ring-panel>', () => {
  it('renders network reach headline', async () => {
    const el = await fixture<ElohimQahalMemberRingPanel>(html`
      <elohim-qahal-member-ring-panel reach="1176"></elohim-qahal-member-ring-panel>
    `);
    expect(el.shadowRoot!.textContent).to.include('1176');
    expect(el.shadowRoot!.textContent).to.include('Network reach');
  });
  it('renders 4 tier sections with counts', async () => {
    const tiers = [
      { id: 'active-stewards-governance', label: 'Active stewards · governance', count: 15, members: [] },
      { id: 'active-stewards-community', label: 'Active stewards · community participation', count: 50, members: [] },
      { id: 'contributor-presences', label: 'Contributor presences', count: 75, members: [] },
      { id: 'compute-hosting-stewards', label: 'Compute-hosting stewards', count: 100, members: [] },
    ];
    const el = await fixture<ElohimQahalMemberRingPanel>(html`
      <elohim-qahal-member-ring-panel reach="240" .tiers=${tiers}></elohim-qahal-member-ring-panel>
    `);
    const sections = el.shadowRoot!.querySelectorAll('section[data-tier]');
    expect(sections.length).to.equal(4);
  });
});
```

- [ ] **Step 2-3: FAIL then implement**

```ts
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';
import { CapabilityAwareElement } from '@elohim/elohim-core/capability';
import './elohim-qahal-imagodei-badge.js';
import './elohim-qahal-provenance-marker.js';

interface MemberTier {
  id: string;
  label: string;
  count: number;
  members: { id: string; name: string }[];
  marker?: 'governance' | 'community' | 'contributor-presence' | 'compute-hosting';
}

/**
 * Member-ring panel — stratified member view with network reach headline.
 *
 * @element elohim-qahal-member-ring-panel
 * @prop {number} reach - Total network reach (headline number)
 * @prop {MemberTier[]} tiers - Stratified tier definitions with members per tier
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings visitor | engaged | contributor | steward | elohim-support
 * @capabilityContentCertainty observed-private
 * @capabilityStates empty:supported, loading:supported, error:supported, stale:supported, contested:n/a, offline:supported, unauthorized:supported
 */
export class ElohimQahalMemberRingPanel extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host { display: block; }
    .headline { font-size: 0.875rem; color: var(--elohim-color-fg-2, #666); }
    .reach { font-size: 2.5rem; font-weight: 600; margin-bottom: 1.5rem; }
    section[data-tier] { padding: 1rem 0; border-top: 1px solid var(--elohim-color-border, #eee); }
    .tier-header { display: flex; justify-content: space-between; align-items: baseline; margin-bottom: 0.5rem; }
    .tier-label { font-weight: 600; font-size: 0.95rem; }
    .tier-count { font-size: 0.95rem; color: var(--elohim-color-fg-2, #666); }
    .member-list { display: flex; flex-wrap: wrap; gap: 0.75rem; }
    .tier-note { font-size: 0.85rem; color: var(--elohim-color-fg-3, #888); margin-top: 0.5rem; }
  `;

  @property({ type: Number }) reach = 0;
  @property({ type: Array }) tiers: MemberTier[] = [];

  override render() {
    return html`
      <div>
        <div class="headline">Network reach</div>
        <div class="reach">${this.reach}</div>
      </div>
      ${this.tiers.map(t => html`
        <section data-tier=${t.id}>
          <div class="tier-header">
            <div class="tier-label">${t.label}</div>
            <div class="tier-count">${t.count}</div>
          </div>
          <div class="member-list">
            ${t.members.slice(0, 8).map(m => html`
              <elohim-qahal-imagodei-badge name=${m.name}></elohim-qahal-imagodei-badge>
            `)}
            ${t.members.length > 8 ? html`<span class="tier-note">+ ${t.members.length - 8} more</span>` : ''}
          </div>
          ${t.id === 'contributor-presences' ? html`<div class="tier-note">non-protocol participants whose recognition accrues to the Qahal commons; in trust until direct participation resolves it</div>` : ''}
          ${t.id === 'compute-hosting-stewards' ? html`<div class="tier-note">lending stewarded compute allocation for resilience, edge distribution, discovery</div>` : ''}
        </section>
      `)}
    `;
  }
}
```

- [ ] **Step 4-8: Register, export, manifest, test, commit**

```bash
git commit -m "feat(elohim-qahal): add elohim-qahal-member-ring-panel (tier-aware, deep-impl)"
```

### Task 4.3 — elohim-qahal-rules-panel

**Purpose:** Renders the Qahal's rubric in human-readable form. Reads from mock-rubrics fixture.

**Files:** standard set

- [ ] **Step 1: Failing test**

```ts
describe('<elohim-qahal-rules-panel>', () => {
  it('renders the rubric name + standing honors', async () => {
    const rubric = { qahalId: 'dowell-household', name: 'Dowell Household — what we honor here', standingHonors: ['care contributed'], bloomMapping: {}, cadenceLabel: '', frictionGradientNote: '', configuredBy: [], lastRevised: '', externalLinkVisibility: {} };
    const el = await fixture<ElohimQahalRulesPanel>(html`
      <elohim-qahal-rules-panel .rubric=${rubric}></elohim-qahal-rules-panel>
    `);
    expect(el.shadowRoot!.textContent).to.include('Dowell Household');
    expect(el.shadowRoot!.textContent).to.include('care contributed');
  });
  it('renders all 6 Bloom-tier mappings when provided', async () => {
    const rubric = { /* with bloomMapping fully filled */ } as any;
    rubric.bloomMapping = { remember: 'R', understand: 'U', apply: 'A', analyze: 'AN', evaluate: 'E', create: 'C' };
    rubric.standingHonors = [];
    const el = await fixture<ElohimQahalRulesPanel>(html`<elohim-qahal-rules-panel .rubric=${rubric}></elohim-qahal-rules-panel>`);
    ['remember', 'understand', 'apply', 'analyze', 'evaluate', 'create'].forEach(t => expect(el.shadowRoot!.textContent!.toLowerCase()).to.include(t));
  });
});
```

- [ ] **Step 2-3: Implement (renders the structured rubric using ASCII-style layout per spec Section 4.3)**

(Full implementation pattern as in 4.1/4.2 — render sections for: name, "Standing in this Qahal is" honors list, Bloom-tier mappings, cadence, friction-gradient note, configured-by, last-revised. ~80 lines of Lit code.)

- [ ] **Step 4-8: Register, export, manifest, test, commit**

```bash
git commit -m "feat(elohim-qahal): add elohim-qahal-rules-panel (deep-impl)"
```

### Task 4.4 — elohim-qahal-co-steward-panel

**Purpose:** Renders the commons-elohim co-steward's observational view in the right-context column. Tone is critical — quiet, declarative.

**Files:** standard set

- [ ] **Step 1: Failing test**

```ts
describe('<elohim-qahal-co-steward-panel>', () => {
  it('renders the steady-state observation', async () => {
    const el = await fixture<ElohimQahalCoStewardPanel>(html`
      <elohim-qahal-co-steward-panel
        primary-observation="The household is steady."
      ></elohim-qahal-co-steward-panel>
    `);
    expect(el.shadowRoot!.textContent).to.include('The household is steady');
  });
  it('renders pending acknowledgments when provided', async () => {
    const pending = ['Sheila\'s recipe', 'Gertrude\'s check-in'];
    const el = await fixture<ElohimQahalCoStewardPanel>(html`
      <elohim-qahal-co-steward-panel primary-observation="x" .pendingAcknowledgments=${pending}></elohim-qahal-co-steward-panel>
    `);
    expect(el.shadowRoot!.textContent).to.include('Sheila\'s recipe');
  });
  it('does not include alarm or notification language', async () => {
    const el = await fixture<ElohimQahalCoStewardPanel>(html`
      <elohim-qahal-co-steward-panel primary-observation="The household is steady."></elohim-qahal-co-steward-panel>
    `);
    const text = el.shadowRoot!.textContent!.toLowerCase();
    expect(text).to.not.include('alert');
    expect(text).to.not.include('warning');
    expect(text).to.not.include('!');
  });
});
```

- [ ] **Step 2-3: Implement**

(Pattern as 4.1 — simple panel rendering primary-observation + pending-acknowledgments list + optional "no urgency" footer. Tone discipline enforced in test.)

- [ ] **Step 4-8: Register, export, manifest, test, commit**

```bash
git commit -m "feat(elohim-qahal): add elohim-qahal-co-steward-panel (deep-impl, observational tone)"
```

### Task 4.5 — elohim-qahal-social-compute-panel

**Purpose:** Renders the resilience topology — selfHub status, stewardsForUs list with health, weStewardFor list, recovery readiness with Shamir threshold. Reads from mock-social-compute-topology.

**Files:** standard set

- [ ] **Step 1: Failing test**

```ts
describe('<elohim-qahal-social-compute-panel>', () => {
  it('renders selfHub status + allocation', async () => {
    const t = { qahalId: 'dowell', selfHub: { hubId: 'dowell-hub', status: 'healthy', allocatedGB: 4, lastSeen: 'now' }, stewardsForUs: [], weStewardFor: [], recoveryReadiness: { ready: true, shamirThreshold: '2/3', lastDrill: '', nextDrill: '' } };
    const el = await fixture<ElohimQahalSocialComputePanel>(html`<elohim-qahal-social-compute-panel .topology=${t}></elohim-qahal-social-compute-panel>`);
    expect(el.shadowRoot!.textContent).to.match(/healthy/);
    expect(el.shadowRoot!.textContent).to.match(/4 GB/);
  });
  it('renders stewardsForUs with health indicators', async () => {
    const t = { /* ... */ } as any;
    t.stewardsForUs = [{ stewardId: 'g', stewardName: 'gertrude', health: 'healthy', percent: 100, lastSync: '17m ago', replicating: 'household state' }];
    const el = await fixture<ElohimQahalSocialComputePanel>(html`<elohim-qahal-social-compute-panel .topology=${t}></elohim-qahal-social-compute-panel>`);
    expect(el.shadowRoot!.textContent).to.include('gertrude');
    expect(el.shadowRoot!.textContent).to.include('17m');
  });
  it('renders recovery readiness with Shamir threshold', async () => {
    const t = { /* ... */ } as any;
    t.recoveryReadiness = { ready: true, shamirThreshold: '2/3', lastDrill: '14 days ago', nextDrill: '16 days' };
    const el = await fixture<ElohimQahalSocialComputePanel>(html`<elohim-qahal-social-compute-panel .topology=${t}></elohim-qahal-social-compute-panel>`);
    expect(el.shadowRoot!.textContent).to.match(/Recovery readiness/);
    expect(el.shadowRoot!.textContent).to.match(/2\/3/);
  });
});
```

- [ ] **Step 2-3: Implement**

(Renders the 4 sections per ASCII mock from spec Section 4.5: self-hub, stewards-for-us with health markers, we-steward-for, recovery readiness line.)

- [ ] **Step 4-8: Register, export, manifest, test, commit**

```bash
git commit -m "feat(elohim-qahal): add elohim-qahal-social-compute-panel (deep-impl, light up the topology)"
```

---

## Phase 5 — Visual-stub panels (4 elements)

These panels are visual-only at MVP — they exist to demonstrate the architectural surface but don't carry deep behavioral content. Each is a placeholder-quality render with the right shape, the right capability profile, and minimal mock data.

### Task 5.1 — elohim-qahal-standing-inspector-panel (visual stub)

**Purpose:** Shows the viewer's standing breakdown in this Qahal — attestation chain walked, affinity weighted, debits subtracted. At MVP: visual placeholder showing the structure but with mocked breakdown numbers.

**Files:** standard set

- [ ] **Step 1: Failing test (visual-stub-quality)**

```ts
describe('<elohim-qahal-standing-inspector-panel>', () => {
  it('renders a standing breakdown structure', async () => {
    const el = await fixture<ElohimQahalStandingInspectorPanel>(html`
      <elohim-qahal-standing-inspector-panel
        .standing=${{ tier: 'contributor', bloomTier: 'apply', attestations: 12, affinity: 0.8, debits: 0 }}
      ></elohim-qahal-standing-inspector-panel>
    `);
    expect(el.shadowRoot!.textContent).to.include('contributor');
    expect(el.shadowRoot!.textContent).to.include('apply');
  });
});
```

- [ ] **Step 2-3: Implement (visual stub)**

```ts
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';
import { CapabilityAwareElement } from '@elohim/elohim-core/capability';
import './elohim-qahal-standing-ring.js';
import './elohim-qahal-capability-tier-chip.js';

interface StandingBreakdown {
  tier: string; bloomTier: string; attestations: number; affinity: number; debits: number;
}

/**
 * Standing inspector panel (visual stub at MVP) — shows the viewer's standing
 * breakdown in this Qahal: attestations walked, affinity weighted, debits subtracted.
 *
 * @element elohim-qahal-standing-inspector-panel
 * @prop {StandingBreakdown} standing
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings engaged | contributor | steward | elohim-support
 * @capabilityContentCertainty observed-private
 * @capabilityStates empty:supported, loading:supported, error:n/a, stale:supported, contested:n/a, offline:supported, unauthorized:supported
 */
export class ElohimQahalStandingInspectorPanel extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host { display: block; }
    h2 { font-size: 1.25rem; font-weight: 600; margin: 0 0 1rem 0; }
    .row { display: flex; justify-content: space-between; padding: 0.5rem 0; border-bottom: 1px solid var(--elohim-color-border, #eee); }
    .stub-note { padding: 1rem; background: var(--elohim-color-stub-bg, #fffbeb); border-radius: 0.375rem; margin-top: 1.5rem; font-size: 0.85rem; color: var(--elohim-color-fg-2, #666); }
  `;

  @property({ type: Object }) standing: StandingBreakdown = { tier: 'visitor', bloomTier: 'remember', attestations: 0, affinity: 0, debits: 0 };

  override render() {
    return html`
      <h2>Standing inspector</h2>
      <div class="row"><span>Tier</span><elohim-qahal-capability-tier-chip tier=${this.standing.tier}></elohim-qahal-capability-tier-chip></div>
      <div class="row"><span>Bloom tier</span><elohim-qahal-standing-ring bloom-tier=${this.standing.bloomTier}></elohim-qahal-standing-ring></div>
      <div class="row"><span>Attestations</span><span>${this.standing.attestations}</span></div>
      <div class="row"><span>Affinity</span><span>${this.standing.affinity}</span></div>
      <div class="row"><span>Debits</span><span>${this.standing.debits}</span></div>
      <div class="stub-note">Visual stub at MVP. Full attestation-chain walk + back-propagation traceback is post-MVP (Sprint 6+).</div>
    `;
  }
}
```

- [ ] **Step 4-8: Register, export, manifest, test, commit**

```bash
git commit -m "feat(elohim-qahal): add elohim-qahal-standing-inspector-panel (visual stub)"
```

### Task 5.2 — elohim-qahal-shefa-resources-panel (visual stub)

Same pattern as Task 5.1. Renders the Qahal's REA flows summary — inflows count, outflows count, commons-share balance, Agreement cascade rules count. Visual placeholder with stub-note explaining post-MVP scope.

- [ ] **Steps 1-8**: as in Task 5.1, with shefa-specific properties (`inflows`, `outflows`, `commonsShareBalance`, `agreementClauses`) and content.

```bash
git commit -m "feat(elohim-qahal): add elohim-qahal-shefa-resources-panel (visual stub)"
```

### Task 5.3 — elohim-qahal-attestations-panel (visual stub)

Same pattern. Renders the viewer's attestation history — quiz results, peer recognitions, contribution validations. Visual placeholder.

```bash
git commit -m "feat(elohim-qahal): add elohim-qahal-attestations-panel (visual stub)"
```

### Task 5.4 — elohim-qahal-graph-discovery-panel (visual stub)

Same pattern. Renders suggestion surface — adjacent Qahals worth knowing, federation candidates. Visual placeholder.

```bash
git commit -m "feat(elohim-qahal): add elohim-qahal-graph-discovery-panel (visual stub)"
```

---

## Phase 6 — Resource list sidebar sections (4 elements)

These mount into the Qahal sidebar's named slots (`panels`, `curated`, `external`, `power-user`) from Phase 3.2.

### Task 6.1 — elohim-qahal-protocol-panel-list

**Purpose:** Lists the protocol panels available in this Qahal. Clicking an item emits an event to switch the active panel.

- [ ] **Steps 1-8**: similar pattern to Task 3.1 (renders a list of items with click handlers, emits `panel-changed` event).

```bash
git commit -m "feat(elohim-qahal): add elohim-qahal-protocol-panel-list resource list section"
```

### Task 6.2 — elohim-qahal-curated-epr-list (with ◆ provenance markers)

Renders curated EPR pointers with `elohim-qahal-provenance-marker category="curated-epr"` icons.

```bash
git commit -m "feat(elohim-qahal): add elohim-qahal-curated-epr-list resource list section"
```

### Task 6.3 — elohim-qahal-external-link-list (capability-gated, offline-greyable)

**Purpose:** Renders web2.0 external links. Filters out links not visible to the viewer's capability tier per the Qahal rubric. Shows greyed/offline state per `navigator.onLine`.

**Files:** standard set

- [ ] **Step 1: Failing test (capability-gating-aware)**

```ts
describe('<elohim-qahal-external-link-list>', () => {
  it('hides links when viewer tier is "child"', async () => {
    const links = [{ id: 'x', url: 'https://example.com', label: 'External' }];
    const rubric = { externalLinkVisibility: { child: 'hidden', steward: 'full' } };
    const el = await fixture<ElohimQahalExternalLinkList>(html`
      <elohim-qahal-external-link-list .links=${links} .rubric=${rubric} viewer-tier="child"></elohim-qahal-external-link-list>
    `);
    expect(el.shadowRoot!.querySelectorAll('a').length).to.equal(0);
  });
  it('shows links when viewer tier is "steward"', async () => {
    const links = [{ id: 'x', url: 'https://example.com', label: 'External' }];
    const rubric = { externalLinkVisibility: { steward: 'full' } };
    const el = await fixture<ElohimQahalExternalLinkList>(html`
      <elohim-qahal-external-link-list .links=${links} .rubric=${rubric} viewer-tier="steward"></elohim-qahal-external-link-list>
    `);
    expect(el.shadowRoot!.querySelectorAll('a').length).to.equal(1);
  });
  it('marks all visible links as external with ⤤ marker', async () => {
    /* ... */
  });
});
```

- [ ] **Step 2-3: Implement (gating logic + offline-grey via `navigator.onLine`)**

- [ ] **Step 4-8: Register, export, manifest, test, commit**

```bash
git commit -m "feat(elohim-qahal): add elohim-qahal-external-link-list (capability-gated, offline-greyable)"
```

### Task 6.4 — elohim-qahal-power-user-expandable

**Purpose:** Section that shows additional panels ONLY if the imagodei settings palette has power-user view enabled. Honors the setting silently — no toggle exposed here.

- [ ] **Steps 1-8**: simple pattern; reads `power-user-enabled` boolean prop (sourced from settings palette in Phase 7), renders the additional power-user panel list items when true, renders nothing when false.

```bash
git commit -m "feat(elohim-qahal): add elohim-qahal-power-user-expandable (honors settings, no toggle)"
```

---

## Phase 7 — Imagodei settings palette + introspection (5 elements)

### Task 7.1 — elohim-imagodei-setting-control

**Purpose:** Primitive for individual setting controls. Renders a setting name, current value, configuration-source (self / steward), and edit affordance gated by capability.

- [ ] **Steps 1-8**: standard pattern with properties for name, value, configurableBy, source.

```bash
git commit -m "feat(elohim-imagodei): add elohim-imagodei-setting-control primitive"
```

### Task 7.2 — elohim-imagodei-protected-tier-marker

**Purpose:** Renders a protected-tier badge on imagodei profiles (child / IDD / elder under guardianship / legal-steward-protected). Reuses provenance-marker patterns but specific to protected tiers.

```bash
git commit -m "feat(elohim-imagodei): add elohim-imagodei-protected-tier-marker primitive"
```

### Task 7.3 — elohim-imagodei-steward-configure-banner

**Purpose:** Banner element shown at the top of the settings palette when a steward is editing a stewardee's settings. Includes an attestation/witness affordance — the changes will be co-signed by the commons-elohim co-steward.

```bash
git commit -m "feat(elohim-imagodei): add elohim-imagodei-steward-configure-banner"
```

### Task 7.4 — elohim-imagodei-settings-palette (composite)

**Purpose:** The full settings palette surface. Composes setting-control elements per the 13-row palette from spec Section 8.2.

- [ ] **Step 1: Failing test**

```ts
describe('<elohim-imagodei-settings-palette>', () => {
  it('renders all canonical setting names from spec Section 8.2', async () => {
    const el = await fixture<ElohimImagodeiSettingsPalette>(html`
      <elohim-imagodei-settings-palette
        viewer-id="matthew-dowell"
        target-id="james-dowell"
        target-tier="child"
      ></elohim-imagodei-settings-palette>
    `);
    const names = [
      'Power-user view',
      'External link visibility',
      'Notification volume',
      'Content reach gating',
      'Standing visibility',
      'Co-steward voice register',
      'Recovery authority delegation',
      'Compute-stewardship visibility',
      'Data export visibility',
      'Language',
      'Web2.0 link click confirmation',
      'Imagodei lens defaults',
      'Onboarding pace',
    ];
    const text = el.shadowRoot!.textContent;
    for (const n of names) expect(text).to.include(n);
  });
  it('shows the steward-configure banner when viewer != target', async () => {
    const el = await fixture<ElohimImagodeiSettingsPalette>(html`
      <elohim-imagodei-settings-palette
        viewer-id="matthew-dowell"
        target-id="james-dowell"
      ></elohim-imagodei-settings-palette>
    `);
    expect(el.shadowRoot!.querySelector('elohim-imagodei-steward-configure-banner')).to.exist;
  });
});
```

- [ ] **Step 2-3: Implement composing the 13 settings + banner + protected-tier marker**

- [ ] **Step 4-8: Register, export, manifest, test, commit**

```bash
git commit -m "feat(elohim-imagodei): add elohim-imagodei-settings-palette (composite, 13 controls)"
```

### Task 7.5 — elohim-imagodei-introspection-panel

**Purpose:** Developer/support-agent surface — shows the full settings state for a human + a "why am I seeing this" trace for each rendered affordance. Visual stub at MVP; full trace primitive is deferred to a Sprint 2 substrate-spine design pass (this plan introduces no new storage entities of its own).

- [ ] **Steps 1-8**: standard pattern. Visual stub with placeholder trace rendering + stub-note explaining the post-MVP introspection trace primitive.

```bash
git commit -m "feat(elohim-imagodei): add elohim-imagodei-introspection-panel (visual stub for support-agent surface)"
```

---

## Self-Review

### 1. Spec coverage

| UX spec section | Implemented by |
|---|---|
| 1. Frame (social garden tending) | Tone discipline in panels (4.4 co-steward "no alarm" test); narrative honored in stream content; member-ring naming |
| 2. Hybrid 4-column chrome | Tasks 3.1-3.4 (4 chrome elements) |
| 3. 9 elohim-core panels | Tasks 4.1-4.5 (deep-impl 5) + 5.1-5.4 (visual stub 4) |
| 4. 5 deep-impl panels | Tasks 4.1-4.5 |
| 5. Configurable resource list | Tasks 6.1-6.4 (4 sidebar sections) |
| 6. Provenance + capability gating | Tasks 1.4 (provenance marker) + 6.3 (external link gating) + 1.3 (capability tier chip with protected-tier marking) |
| 7. Mock-data fixtures | Tasks 2.1-2.4 (4 fixture modules) |
| 8. Settings palette | Tasks 7.1-7.5 (5 imagodei elements) |
| 9. Architectural connections | Capability profile JSDoc captures these per element |
| 10. Sprint 1 deliverables | This plan IS Plan A for Sprint 1; Plan B (Library B stories) deferred |
| 11. Open questions | UX-spec section 11 lists 10 items; this plan implements none of them directly (they are substrate concerns, not UX-element concerns) and introduces no new storage entities of its own |

**Gaps:** No tasks for Library B designed pattern stories (canonical scenes + variations + capability-gating + user-toggle stories), Storybook 0.0.0.0-binding configuration, or comprehensive a11y/i18n/ua-prefs deeper testing (each task includes basic axe-core checks but the full triple-gate test suite is reduced for this plan's scope). These belong in **Plan B — Library B graphos pattern stories** (next plan to be written after Plan A completes).

### 2. Placeholder scan

- No "TBD" or "implement later" in active text.
- Visual-stub panels (Phase 5) are honestly labeled — they have a `stub-note` in the rendered output explaining post-MVP scope. Not a planning placeholder; an architectural honesty.
- Tasks 4.3, 4.4, 4.5 mark "(Full implementation pattern as in 4.1/4.2 — render sections for: ...)" — these reference the established pattern but don't say "Similar to Task N." They give specific content to render. Acceptable, but watch during execution: each task gets fresh Lit code, no copy-paste shortcuts.

### 3. Type consistency

- `StandingTier` (Task 1.1), `BloomTier` (Task 1.2), `CapabilityTier` (Task 1.3), `ProvenanceCategory` (Task 1.4), `CareEconomyKind` (Task 1.5) — all unique, no naming collisions.
- `MockImagodeiProfile.standingTier` matches the badge's `standingTier` enum.
- `MockImagodeiProfile.bloomTier` matches the standing-ring's `BloomTier`.
- `MockImagodeiProfile.capabilityTier` matches the chip's `CapabilityTier`.
- Stream events shape (Task 4.1) matches `MockCareEconomyEvent` (Task 2.3).
- Topology shape (Task 4.5) matches `MockComputeTopology` (Task 2.4).

Consistent.

---

## Execution handoff

**Plan complete and saved to `/projects/elohim/genesis/docs/plans/2026-05-22-sprint-1a-elohim-elements-plan.md`.**

This plan covers Sprint 1 Phase A — the elohim-elements work (28 elements + 4 fixture modules across 32 tasks). Plan B (Library B designed pattern stories + Storybook integration) is the next plan to author; it depends on this plan's deliverables.

**Two execution options:**

1. **Subagent-Driven (recommended)** — Dispatch a fresh subagent per task, review between tasks, fast iteration. Each task is bounded (1 element + spec + manifest spec + commit), well-suited for subagent execution with the component-architect specialty.

2. **Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints. Suitable if you want to drive the pace yourself.

Which approach?
