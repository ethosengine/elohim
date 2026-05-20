# elohim-core

Protocol substrate — tokens, light-DOM globals, and atomic Custom Elements.

## What's here

- `tokens.scss`, `base.scss`, `animations.scss` — Layer 1 substrate (CSS custom properties + light-DOM globals)
- `src/elohim-button.ts` — first Lit atom; reference shape for all future atoms
- `src/register.ts` — side-effectful entry that registers all elements
- `src/index.ts` — side-effect-free re-exports for type imports

## Build

```bash
pnpm --filter elohim-core run build
```

Produces `dist/{index.js, register.js, *.d.ts, custom-elements.json}`.

## Test

```bash
pnpm --filter elohim-core run test
```

Runs functional + a11y unit tests in chromium via web-test-runner. Includes 15 component behavior tests + 7 manifest verification tests.

## Lint, format, style

```bash
pnpm --filter elohim-core run lint        # ESLint flat config (sonarjs/unicorn/lit/wc)
pnpm --filter elohim-core run lint:css    # Stylelint (postcss-lit on css`` blocks + SCSS)
pnpm --filter elohim-core run format      # Prettier write
pnpm --filter elohim-core run format:check
```

Configs live at the umbrella `app/elohim-elements/` so all 8 packages inherit one source of truth.

## Capability Profile

`elohim-core` exports the **Capability Profile** primitive — a frozen context object that every element observes via the `CapabilityAwareElement` mixin. The profile carries:

- `lens` — disclosure tier (minimal → trace)
- `theme` / `contrast` / `locale` — visual + language register
- `stimulus` — motion tier (**default: still**)
- `textuality` — symbolic vs textual
- `standings[]` — attested roles
- `lock` — steward / elohim-support / pilot constraint set
- `origin` — who set this profile

Companion `ContentCertainty` describes the **content being rendered**: canonical / partial / stale / contested / unreachable / unknown.

Elements declare a `capabilityContract` via JSDoc `@capability*` tags, which the CEM analyzer reads into `dist/custom-elements.json`. The contract is the single source of truth for *which cells the element implements* and *which states it has designed for*.

Three **precondition gates** sit above all cells — a11y, i18n, ua-prefs (incl. photosensitive-flash). Failing any gate blocks every cell.

For the full design, see `genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md`.

## Adding a new atom

1. **Write the failing test in `src/<your-atom>.spec.ts`** — describe the public contract (slot, properties, events, ARIA, axe-core scan). Tests are the spec; the implementation must satisfy them.
2. **Implement `src/<your-atom>.ts`** extending `LitElement`. Use:
   - JSDoc tags (`@element`, `@prop`, `@fires`, `@slot`, `@cssprop`, `@csspart`) for the manifest analyzer
   - `@capabilityMaxLens`, `@capabilityThemes`, `@capabilityContrast`, `@capabilityLocales`, `@capabilityMaxStimulus`, `@capabilityTextuality` for the visual contract
   - `@capabilityRequiredStandings`, `@capabilityOptionalStandings` for role requirements
   - `@capabilityContentCertainty observed | not-observed` and `@capabilityStates name:status, ...` for the orthogonal claims
   - `@property` decorators (NOT `@customElement` — registration lives in register.ts to keep index.ts side-effect-free)
   - `static readonly` on `shadowRootOptions` and `styles` (sonarjs lint rule)
   - `delegatesFocus: true` on `shadowRootOptions` for focusable atoms
3. **Add the export to `src/index.ts`** and the `customElements.define` call to `src/register.ts`.
4. **Add a manifest assertion in `src/<your-atom>.manifest.spec.ts`** — verify the CEM analyzer picked up tag, properties, events, slots, cssProperties, cssParts.
5. **Add a `*.stories.ts` in graphos** under `foundations/__docs__/components/`.
6. **Run `pnpm run elements:codegen`** (regenerates manifest) and commit `dist/custom-elements.json`.

## Tag naming convention

- **Core atoms** (this package): flat prefix — `<elohim-button>`, `<elohim-card>`, `<elohim-input>`, etc.
- **Pillar components** (e.g., `elohim-imagodei`): pillar-namespaced — `<elohim-imagodei-login>`, `<elohim-lamad-content-viewer>`, etc.
- **Third-party authors** follow the same pattern with their own vendor prefix: `<acme-button>`, `<acme-skills-quiz>`.

The W3C Custom Elements spec requires a hyphen and recommends a vendor prefix; we always vendor-prefix to scale cleanly to third-party pillar implementations.

## Side-effect contract

`./index.ts` is side-effect-free — `import { ElohimButton } from 'elohim-core'` does NOT register the element. Consumers must `import 'elohim-core/register'` explicitly.

This avoids a tree-shaking trap: with `@customElement` decorator, an unused class import would still register the element via module evaluation, but a tree-shaker dropping the unused import would also drop the registration. Separating the side-effect from the class import makes the contract explicit.

## Why decisions are this way

See `genesis/docs/superpowers/specs/2026-05-06-elohim-lit-component-pivot-design.md`.

For the implementation plan: `genesis/docs/superpowers/plans/2026-05-06-elohim-lit-component-pivot-plan.md`.
