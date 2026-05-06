# Elohim Lit Component Layer Pivot — Design

**Date:** 2026-05-06
**Status:** Design (awaiting review)
**Scope:** Pivot the elohim component layer from SCSS-only modules to Lit-based Web Components
**First-sprint deliverable:** One end-to-end working atom (`<elohim-button>`) proving the pattern

---

## Why pivot

The Elohim Protocol is meant to outlive any single frontend framework. Today's component candidates live in `app/elohim-app/src/app/**` as Angular components — 178 of them, framework-coupled. The pivot moves the component substrate to W3C Custom Elements via Lit so:

1. **Reference implementations of pillars are framework-agnostic.** The pillars (imagodei, lamad, shefa, qahal, doorway, avodah) are reference implementations of the protocol, not Angular features. Third parties who implement their own pillar surfaces should consume the same primitives without inheriting Angular.
2. **Layer 2 (W3C standards) over layer 5 (framework-coupled).** Lit is a thin LitElement base class plus tagged-template html/css. It compiles to standard Custom Elements. Other frameworks (Angular today, React/Vue/Svelte tomorrow) consume them identically.
3. **`<sophia-question>` already validates the WC boundary in this stack.** The doorway from rendering library to Angular host is proven; this pivot generalizes it.
4. **Long-arc thin-client move.** Angular `.component.ts` shells become thin wrappers around the WC. Eventually they disappear, and the pillar's "view" is the published custom-elements bundle.

## Layer model

The substrate has three layers. Each has a clear concern and a clear consumer.

| Layer | Concern | Where it lives | Consumer |
|---|---|---|---|
| **1 — Tokens & light-DOM globals** | CSS custom properties, body bg, typography defaults, animation `@keyframes` (which can't live inside shadow DOM by spec) | `elohim-core` SCSS files | Apps + components |
| **2 — Custom Elements** | Lit components. Per-pillar packages. Encapsulate styles via `static styles = css\`...\``. Consume tokens via `var(--*)`. | `app/elohim-elements/{core,imagodei,lamad,shefa,qahal,doorway,avodah,shell}` | Apps |
| **3 — Composition surface** | Storybook for documenting and composing primitives. Stories, MDX, prototype sketches. Where lamad-ui-style design explorations graduate from. | `app/elohim-library/projects/graphos` | Designers, contributors |

The earlier `elohim-styles` / `graphos` split was the right intuition; what's been clarified is that styles + components are the **same layer 1+2 substrate**, not separate concerns. Tokens and the components consuming them evolve in lockstep, so they share a package.

## Decisions locked

### D1. Package shape — one per pillar, atoms in core

Eight packages, mirroring the existing scaffold and the canonical pillar boundaries from `app/elohim-app/src/app/`:

```
app/elohim-elements/
  elohim-core/        — tokens.scss + light-DOM globals + cross-pillar atoms (button, card, input, badge, …)
  elohim-shell/       — landing and host chrome
  elohim-imagodei/    — identity pillar
  elohim-lamad/       — learning pillar
  elohim-shefa/       — economy pillar
  elohim-qahal/       — community pillar
  elohim-doorway/     — gateway integration pillar
  elohim-avodah/      — protocol-as-process meta-pillar
```

`elohim-core` houses tokens and atoms (a button has no pillar). Pillar packages house pillar-specific molecules and organisms (login flow, content viewer, banking dashboard). Cross-pillar atom = signal it belongs in core, never duplicated.

**Why not split atoms/molecules into separate tiers:** would create a structural divergence between the component library and the consuming app's `src/app/` layout. CLAUDE.md treats pillar boundaries as canonical; honor that single axis.

**Why not one mega-package with subpath exports:** couples release cycles, obscures the boundary. Per-pillar packages let pillars evolve independently. The README at `app/elohim-styles/README.md` already commits to this reasoning.

### D2. Custom element tag naming — hybrid prefixed

- **Core atoms:** `<elohim-button>`, `<elohim-card>`, `<elohim-input>` (no pillar segment)
- **Pillar components:** `<elohim-imagodei-login>`, `<elohim-lamad-content-viewer>`, `<elohim-shefa-banking-panel>`
- **Tags mirror package names 1:1** — `elohim-imagodei` package → `<elohim-imagodei-*>` tags. Easy to find source from tag.
- **Third parties follow the pattern:** `<acme-button>`, `<acme-skills-quiz>`. Always vendor-prefixed; no namespace contention with elohim's reference implementations.

**Why not flat (`<elohim-login>`):** ambiguous. Login *what*? Imagodei? Doorway? Account-recovery?

**Why not drop the `elohim-` prefix on pillar tags:** pillar names become a quasi-trademarked global namespace; a 3rd-party's `<imagodei-login>` would collide with elohim's reference impl. W3C best practice is always-vendor-prefix.

### D3. Build tooling — Vite library mode

Each package builds via `vite build --lib`. Shared root `vite.config.lib.ts` + per-package `vite.config.ts` overrides for entry points and tag declarations.

Output per package:
- `dist/index.js` (ESM, side-effect-free)
- `dist/register.js` (side-effectful — registers all tags)
- `dist/index.d.ts`
- `dist/custom-elements.json` (CEM manifest, post-build step)

**Why Vite over rollup/tsup/unbuild:**
- Stack already uses Vite/Vitest in elohim-app — devs reuse mental model
- Built-in library mode + d.ts emit; minimal config
- Free Vite preview for sandboxed component dev outside Storybook
- TS-native, ESM-native, fast

### D4. Storybook framework — replace `@storybook/angular` with `@storybook/web-components`

No `.stories.ts` files exist today, only framework-agnostic MDX. Switching is essentially free now and gets harder every commit.

- Existing MDX docs continue to work
- All future stories are WC-native
- One storybook instance, single dev server
- The 5 lamad-ui Angular design explorations stay as-is until claimed; if they need stories, they get rewritten as Lit at that point

**Why not stand up a second storybook instance:** two configs, two ports, navigation split — needless cost when no Angular stories exist.

**Why not multi-framework hack:** Storybook 10 still selects one framework per `.storybook/main.ts`; multi-framework instances require fragile bridge hacks the Storybook team warns against.

### D5. Umbrella directory rename — `app/elohim-styles/` → `app/elohim-elements/`

Cheap now (8 packages, mostly placeholder; `pnpm-workspace.yaml` references them as the only consumers). Permanent semantic mismatch if deferred. Each package houses both styles (where they exist) and components — no internal styles/components boundary.

### D6. Style pattern — canonical Lit

- `static styles = css\`...\`` inside each component (auto-uses adopted stylesheets)
- Tokens via `var(--lamad-bg-primary)` etc. — penetrates shadow DOM
- Light-DOM globals (body bg, typography, `@keyframes`) stay in `elohim-core` SCSS exports
- No shadow-DOM-busting tricks (`::part`/`::slotted` only when the component intentionally exposes them; otherwise encapsulation is the contract)

### D7. JSDoc conventions — required for manifest

Every component documents its public surface with CEM-readable JSDoc tags. Missing tags = empty manifest entries = blind consumers.

```typescript
/**
 * The elohim button atom — substrate primitive.
 *
 * @element elohim-button
 *
 * @prop {('primary'|'secondary'|'ghost')} variant - Visual variant
 * @prop {boolean} disabled - Disabled state
 *
 * @event {CustomEvent<void>} elohim-click - Fired on activation
 *
 * @slot - Default slot for label content
 *
 * @cssprop --elohim-button-bg - Override background
 * @csspart label - The label container
 */
```

### D8. Manifest tooling + CI hook

- `@custom-elements-manifest/analyzer` runs as post-build per package
- Emits `dist/custom-elements.json`
- Pre-push hook (mirroring the existing `schema:codegen` pattern) checks manifest freshness against source hash; fails with `pnpm run elements:codegen` if stale
- **Forward-affordance:** the manifest schema reserves a `componentCid` field (computed-by-tooling) for future federation/integrity use. Not implemented in this sprint, but the slot exists so downstream tooling can populate it without a manifest schema break.

### D9. A11y testing

- Storybook `addon-a11y` (already installed) for visual a11y panels in storybook
- Unit-level: `@web/test-runner` with `axe-core` plugin per package; `*.spec.ts` colocated with source
- Tab order, keyboard navigation, focus management are first-class component behaviors, not afterthoughts

### D10. Migration strategy for the 178 elohim-app components — opportunistic

- **Not** a forced-march sprint to convert all 178
- When a component is touched for any reason (bug, feature, refactor), evaluate whether to migrate
- Substrate quality and proof-of-pattern matter more than migration headcount
- The pillar boundary violations backlog (174 violations, per memory) is a separate concern; that cleanup may opportunistically extract into Lit during the same touch

### D11. lamad-ui — stays as-is

5 Angular design-exploration components in `app/elohim-library/projects/lamad-ui`. Not migrations. Will be rewritten as Lit if/when claimed by a real surface; otherwise they remain prototype Angular. Storybook (now WC) won't host them. Decision deferred to per-component basis.

---

## First-sprint scope — `<elohim-button>` end-to-end

The full proof loop. Once it closes, replication for additional atoms and pillar components is mechanical.

### Deliverables

1. **Directory rename:** `app/elohim-styles/` → `app/elohim-elements/`. Update `pnpm-workspace.yaml` and `app/elohim-elements/README.md` to reflect the unified scope.
2. **`elohim-core/elohim-button.ts`** — first Lit component:
   - Class `ElohimButton extends LitElement`
   - Reactive `variant`, `disabled` properties
   - Default slot, focus management, ARIA disabled handling, hover/active/focus tokenized states
   - JSDoc tags for the manifest
3. **`elohim-core/src/register.ts`** — side-effectful entry that registers `<elohim-button>` (via `customElements.define`)
4. **`elohim-core/vite.config.ts`** — library build config (ESM + d.ts + CSS)
5. **`elohim-core/custom-elements-manifest.config.mjs`** — CEM analyzer config
6. **Build script (`pnpm --filter elohim-core build`)** — produces `dist/{index.js, register.js, index.d.ts, custom-elements.json}`
7. **Storybook framework swap** — `app/elohim-library/.storybook/main.ts` from `@storybook/angular` to `@storybook/web-components`; preview imports `elohim-core/register`
8. **`graphos/foundations/__docs__/components/elohim-button.stories.ts`** — first WC story showing variants, dark/light, disabled
9. **One consumer in elohim-app** — `app/elohim-app/src/app/components/not-found/not-found.component.html` (three `<button>` elements with `btn-primary` / `btn-secondary` / `btn-ghost` classes) becomes three `<elohim-button>` elements with corresponding `variant` props. Add `CUSTOM_ELEMENTS_SCHEMA` to the host module/component. The not-found page exercises all three variants in one swap, has low traffic (low blast radius), and is fully isolated.
10. **Pre-push hook** — `tools/check-cem-fresh.mjs` (mirrors `schema:codegen` pattern); fails if `custom-elements.json` doesn't match source hash
11. **A11y test** — `elohim-button.spec.ts` exercising keyboard activation, focus visibility, ARIA disabled, axe-core scan

### Out of scope for this sprint

- All 7 non-core pillar packages stay placeholder (no Lit components yet)
- No additional atoms beyond `<elohim-button>` (card/input/badge — iteration 2)
- No pillar tokens (e.g., `--imagodei-*` namespace) — only the existing `elohim-core` tokens
- No shrinking of Angular `.component.ts` shells — that's iteration 3+
- No CID computation — manifest reserves the field but nothing populates it
- No migration of any of the 178 elohim-app components beyond the single consumer touch

### Acceptance — the loop closes

- `pnpm --filter elohim-core build` produces a complete `dist/`
- `pnpm storybook` (in elohim-library) renders the `<elohim-button>` story with variants, dark/light, hover/focus
- `pnpm --filter elohim-app start` runs; navigating to a 404 URL renders the not-found page with three `<elohim-button>` elements in primary/secondary/ghost variants, all clickable and keyboard-activatable
- `pnpm --filter elohim-core test` passes (axe + functional)
- Pre-push hook fails when source changes without manifest regen, passes after `pnpm run elements:codegen`
- `git mv` of `elohim-styles` → `elohim-elements` is clean; no broken imports

---

## Architecture summary

```
app/
  elohim-elements/                     ← layer 1+2 substrate (renamed from elohim-styles)
    elohim-core/                       ← tokens + atoms
      tokens.scss
      base.scss
      animations.scss
      src/
        elohim-button.ts               ← first proof
        register.ts                    ← side-effectful registration
        index.ts                       ← re-exports (side-effect-free)
      vite.config.ts
      custom-elements-manifest.config.mjs
      package.json
      dist/                            ← built artifact (gitignored)
        index.js
        register.js
        index.d.ts
        custom-elements.json
    elohim-shell/                      ← placeholder (no Lit yet)
    elohim-imagodei/                   ← placeholder
    elohim-lamad/                      ← placeholder
    elohim-shefa/                      ← placeholder
    elohim-qahal/                      ← placeholder
    elohim-doorway/                    ← placeholder
    elohim-avodah/                     ← placeholder
  elohim-library/
    .storybook/main.ts                 ← @storybook/web-components (was @storybook/angular)
    projects/
      graphos/src/foundations/__docs__/components/
        elohim-button.stories.ts       ← first WC story
      lamad-ui/                        ← unchanged (5 Angular explorations)
      ...
  elohim-app/
    src/app/
      <some-host>.module.ts            ← CUSTOM_ELEMENTS_SCHEMA
      <some-host>.component.html       ← <elohim-button> usage
tools/
  check-cem-fresh.mjs                  ← pre-push hook
```

## Risk register

| Risk | Mitigation |
|---|---|
| Vite library mode + d.ts emit has rough edges with some plugins | Use `vite-plugin-dts` (mature, Lit ecosystem standard); fall back to running `tsc --emitDeclarationOnly` separately if needed |
| `CUSTOM_ELEMENTS_SCHEMA` weakens Angular template type-checking globally for the host module | Scope it to a single host component, not the whole app module; future Angular CDK/zoneless work will reduce the cost |
| CEM analyzer misses JSDoc tags if tagged incorrectly | Add a unit test (`elohim-button.manifest.spec.ts`) that loads the generated `custom-elements.json` and asserts expected `properties`, `events`, `slots`, `cssProperties` entries; runs in CI |
| Storybook 10 framework switch breaks existing MDX | MDX docs are framework-agnostic; verify by running storybook before and after the switch |
| Pre-push hook adds friction | Keep the check fast (< 200ms); use `HUSKY=0` escape hatch already documented in CLAUDE.md |

## What replication looks like (post-sprint, not in scope)

Once the loop is proven:
- Add `<elohim-card>`, `<elohim-input>`, `<elohim-badge>` to `elohim-core/src/` — each copies the proven shape (component file, register entry, story, manifest test)
- When a pillar surface needs a primitive, author it in `elohim-{pillar}/src/` — same pattern
- When a component is touched in `elohim-app`, evaluate Lit migration; bring it across opportunistically
- The manifest's `componentCid` slot fills in once federation/integrity tooling lands (separate spec)

## References

- `app/elohim-styles/README.md` — current SCSS-only intent (will be rewritten)
- `app/elohim-app/src/styles.css` — original styles, partially harvested
- `app/elohim-library/.storybook/main.ts` — current `@storybook/angular` config
- `sophia/packages/sophia-element/` — existing WC distribution (React + manual wrap, not Lit; pattern-only reference for Angular consumption via CUSTOM_ELEMENTS_SCHEMA)
- `CLAUDE.md` (root + `app/elohim-app/CLAUDE.md`) — pillar boundary guardrails
- `pnpm-workspace.yaml` — workspace registry to update on rename
