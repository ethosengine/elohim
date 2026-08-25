---
id: elohim-elements-ui-substrate-gospel
cites:
  - "elohim-library-pattern-gospel | story-level binding discipline — Library A/B boundary; Library B decorators are Storybook-only until graphos-tokens ships | sha256:94b851810ce6cdc8 | status: stale — target content moved on; re-verify | path: app/elohim-library/CLAUDE.md"
  - "elohim-app-frontend-gospel | the shell side of the layer rails — protocol-omni trust surface, ThemeService twin contract, EPR-native cross-bundle nav | sha256:5310d1b5bad40d86 | status: stale — target content moved on; re-verify | path: app/elohim-app/CLAUDE.md"
  - "lamad-bundle-gospel | the bundle-consumer side of the layer rails — B18 token wiring is the worked example | sha256:1bc6eb8e1c112bc4 | status: stale — target content moved on; re-verify | path: app/lamad/CLAUDE.md"
  - genesis/data/timeline/backlog/bundle-styling-token-contract.md
  - "omnibar-consolidation-epr-native-links-design | the design whose theme/serving-context/nav decisions the layer rails enforce; records the B18 styling audit | sha256:92df16eea8d9bcf8 | status: stale — target content moved on; re-verify | path: genesis/docs/superpowers/specs/2026-06-05-omnibar-consolidation-epr-native-links-design.md"
  - "elohim-seam-map-concern-routing | the concern-routing atlas — this surface owns the client surface seam (§3.8); routes any where-does-this-go? question | sha256:7fd48274fae5e8c5 | status: stale — target content moved on; re-verify | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md"
---

# elohim-elements — UI Substrate Gospel

This document is the **shared synthesis** for everyone (humans + agents) authoring inside `app/elohim-elements/`. It establishes scope discipline: what this directory is for, what it is NOT for, and the anti-patterns that creep in when those lines blur.

**Agents that read this:** `component-architect` (primary), `angular-architect` (when migrating Angular components into Lit primitives), `graphos-designer` (when binding brand tokens — reads from here).

For operational structure (modules, layer model, dependency direction, tag naming), see `README.md` in this directory. This file is gospel about scope.

---

## Seam map — you are here

This surface owns the **client surface** seam (atlas §3.8 — presentation + user-intent capture: stateless Lit primitives render host-provided state accessibly across the lens gradient and emit intent events).

Any "where does this go?" concern routes through the concern-routing atlas: `elohim-seam-map-concern-routing`.

Confusion-to-avoid: client vs doorway projection — a "content 404 / blob missing" is a substrate replication problem, not a UI bug; and the Capability-Profile lens gradient (a render concern) ≠ hardware-tier detection (a node concern, §3.1).

---

## What lives here

This directory hosts **stateless Lit Custom Elements** — the protocol's blank-slate UI substrate. The contract every element honors:

| Concern | Where it lives in an element |
|---|---|
| **Visual + structural rendering** | `static styles = css\`…\`` + `render()` |
| **Accessibility** | ARIA roles, keyboard handlers, focus management, semantic HTML |
| **Sense-and-respond** | `@property` setters reacting to host-provided state |
| **User-action capture** | Custom event dispatch (`@fires reaction-submit`, `@fires graduated-feedback`, …) |
| **Theme + locale + stimulus + contrast adaptation** | Capability Profile contract (`@capability*` JSDoc tags) |
| **Override surface** | CSS custom properties published as `@cssprop` |

Every element answers the substrate-as-steward question: *given pre-computed state from somewhere, how do I render it accessibly, internationally, across the capability gradient, and emit user signals back?*

The three legitimate client-side concerns elements own:

1. **UX** — what the user sees, how it looks, how it behaves visually
2. **Accessibility** — keyboard nav, screen reader semantics, contrast, focus, motion preferences
3. **Sense-and-respond** — reacting to host-provided state changes, emitting events about user intent

That is the whole job. Nothing else.

---

## What does NOT live here (anti-patterns)

If you find yourself adding any of the following to a Lit element, **stop**. It's a smell. The work belongs on the backend (doorway route, zome coordinator, or storage projection), not in any client-side library — and especially not in a stateless UI primitive.

| Anti-pattern | Substrate-correct home |
|---|---|
| **API calls to substrate endpoints** (`fetch('/api/v1/...')`, HTTP clients, holochain client calls) | Doorway route handler or zome coordinator; the element gets pre-computed data as `@property` |
| **REA economic-event creation** (signing, validating, projecting EconomicEvent/Commitment/FeedbackSignal) | Zome coordinator function; doorway publishes the projection; element just emits a user-intent event |
| **Aggregated state derivation across multiple substrate entries** | Storage projection layer (Rust); element receives a single view |
| **Submission flow orchestration** (multi-step form coordination, retry, error reconciliation) | Doorway route owns the transaction; element renders submit / busy / success / error states from a single `@property` |
| **Mediation / consent / governance logic** (deciding what mechanism applies, validating eligibility, computing weights) | Zome coordinator + storage projection; element receives `MechanismSelection` / `AccumulationStatus` views and renders them |
| **Recognition / standing / mastery wiring** (computing weights, distributing affinity, recording attestations) | REA primitive in zome (`mishpat::Commitment`); element only emits the user-action event that triggers it |
| **Cache / persistence / pre-fetch** (IndexedDB writes, localStorage of substrate state, manual cache invalidation) | `elohim-storage` cache layer; element subscribes to view changes via host-provided signal |
| **Authentication or session management** | `@elohim/identity` (host-provided) — element receives the identity-shaped data; doesn't decide whether the user is authenticated |
| **Cross-element coordination** (one element manipulating another's internals, shared mutable singletons) | Composition is the host's job — host wires events between elements; elements don't reach across |

The rule of thumb: **if removing the element from the page would leave a piece of business logic somewhere it shouldn't be, it was in the wrong place.** Lit elements own how things look and how user signals get emitted. Everything else is somewhere else.

### The thin-client discipline (substrate-as-steward)

The substrate ([stewardship-over-sovereignty](../genesis/docs/architecture/stewardship-over-sovereignty.md) §3) is the steward. The substrate owns truth — Holochain DHT entries, elohim-storage projections, doorway routes. When state aggregation, validation, or transaction orchestration lives client-side, the client begins to own responsibilities the substrate should hold. That breaks:

- **Cross-host consistency** — three different Lit-element instances on three devices compute the same aggregation three different ways
- **Offline correctness** — client orchestration assumes connectivity; substrate-mediated flows degrade gracefully
- **Auditability** — REA events created client-side bypass the substrate's notarization story
- **Multi-client uniformity** — `doorway.elohim.host` (browser), Tauri desktop, third-party clients all render the SAME elements; if elements own orchestration, every client re-implements it
- **The five-library SDK boundary** — `@elohim/service`, `elohim-core`, `@elohim/storage-client`, `@elohim/identity`, `@elohim/rea-runtime` are operational consumption surfaces (Category C). They hold no source-of-truth state and don't orchestrate substrate transactions. Anything that LOOKS like state-owning code on the client is a sign the substrate owes a new route or zome function

When you encounter a stateful Angular component that should be migrated to a Lit element, the **scope-shift moment** is recognizing which lines are UX/a11y/sense-and-respond (move into the Lit element) and which lines are orchestration (open a backend-migration ticket; the Lit element stays stateless; the orchestration moves to doorway/zome).

See `genesis/docs/architecture/pillar-bundle-split-runbook.md` §6.14 for the worked example (Slice 2.2b — three @app/qahal Angular components encapsulate ~1650 lines of orchestration that should not exist in any client-side library).

---

## The Capability Profile contract

Every element declares its claims via `@capability*` JSDoc tags:

```ts
/**
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en, es, he
 * @capabilityMaxStimulus still
 * @capabilityTextuality text-and-icon
 * @capabilityStandings novice, journeyman, mentor
 */
```

The Capability Profile is a frozen context object naming `lens × theme × contrast × locale × stimulus × textuality × standing`. Substrate-honored rendering means **the same primitive renders the same view differently across the gradient — never hiding what a lower lens shows, only revealing**. A child sees `simple`; a kernel developer sees `trace`; the element is the same; the consumer code is the same. See `genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md` for the substrate primitive.

Elements without a Capability Profile declaration are not eligible for `elohim-core`.

---

## The four precondition gates

Before claiming a Lit element is ready, every element passes four gates:

1. **Accessibility (a11y)** — ARIA roles correct, keyboard navigation complete, focus management, screen reader semantics tested. Default story includes accessibility-tree assertions.
2. **Internationalization (i18n)** — lit-localize hooks wired; locale-aware formatting; right-to-left support for he. No hardcoded English strings.
3. **User-agent preferences (ua-prefs)** — `prefers-color-scheme`, `prefers-reduced-motion`, `prefers-contrast` honored. Stories demonstrate each preference's effect.
4. **Theme contrast (theme-contrast)** — the `@capabilityThemes`/`@capabilityContrast` claims are EXECUTABLE: every claimed theme has a passing gate cell (`src/testing/theme-contrast.ts`). All claimants run the **system cells** (blank-slate under `color-scheme: light|dark` — system-color defaults must pair correctly per scheme); elements with a shipped binding also run the **tokens cells** (the REAL `tokens.scss` + binding injected as the fixture, `documentElement[data-theme]` set — the production cascade, never a copy) plus the **reactivity canary** (themed surfaces must CHANGE across themes; a frozen var-chain is the 2026-06-05 dark-mode regression class). The contrast walk is computed-style WCAG 1.4.3/1.4.11; axe runs strict (violations AND color-contrast incompletes empty, content/parse abstentions excused). An element that cannot pass a cell shrinks its claim — never skips the assertion. Fixtures bind tokens in the TEST, never in the element.

The four gates are non-negotiable. An element that fails any one is not contract-passing.

---

## Blank-slate discipline

`elohim-core` elements are theme-agnostic. They expose `@cssprop --elohim-*` custom properties but never bind brand defaults. The Elohim brand is one consumer among many — `graphos-designer`'s Library B binds it via story decorators, never below.

Brand bake in element CSS is the cardinal sin. Replacing `var(--elohim-button-bg, Canvas)` with `var(--elohim-button-bg, #ef6c00)` ships the brand inside the primitive and breaks every downstream binding.

Default values use CSS system colors (`Canvas`, `CanvasText`, `ButtonFace`, `ButtonText`, `LinkText`, `Mark`, `MarkText`, …) and `font: inherit`. The blank-slate proof story (`style="all: initial;"`) is the verification: if the element looks like a usable raw HTML control with zero tokens bound, the discipline held.

---

## Layer rails — component → UI composition (one concern per layer)

The chain from element to rendered, branded UI has exactly four layers. Each has ONE home; blending them is how the B18 gap happened (tokens harvested into `tokens.scss` but imported by nothing — lamad shipped 575 unresolved `var(--lamad-*)` references; 2026-06-05 styling-migration audit).

| Layer | Home | The one concern |
|---|---|---|
| **Element** | `<module>/src/*.ts` (this directory) | Render blank-slate + a11y + emit intent. Publishes `@cssprop`; binds nothing. |
| **Token layer** | `elohim-core/tokens.scss` | Palette + theme reactivity (`:root[data-theme]`, `prefers-color-scheme`) + `color-scheme` (system colors must always agree with the palette). Defined once; only SHIPS when a bundle imports it. |
| **Binding layer** | per-bundle `_chrome-binding.scss` (interim) → graphos-tokens artifact (canonical, unbuilt) | Map element `--elohim-*` cssprops onto the palette. Nothing else lives in that file. |
| **Bundle** | `app/<bundle>/src/styles.scss` | IMPORTS the layers + genuinely bundle-local styles. Never defines or duplicates tokens. |

Rails:
- An element never references a token it doesn't declare as `@cssprop` with a system-color default.
- A bundle whose components consume `var(--lamad-*)` (or any palette) MUST import the token layer — "it renders fine in the shell" is not evidence (the shell's global `styles.css` is doing the work).
- Preference state crosses the chain only via the shared store contracts (`theme/theme-store.ts`: `localStorage['elohim-theme']` + `html[data-theme]` (AUTHORITY — token overrides + `color-scheme` live at `:root[data-theme]`, where the `:root`-declared binding chains actually re-resolve; a custom property's `var()` refs substitute at the DECLARING element, so a body-level override can never reach a `:root`-declared chain) + `body[data-theme]` (legacy compat, dual-written) + `elohim-theme-changed`; `localize/locale-store.ts` likewise) — never element-private theme/locale state.

Concern routing (content-addressed — slugs resolve via this file's `cites:` frontmatter and survive moves):
- `elohim-library-pattern-gospel` — story-level binding discipline; Library B decorators are Storybook-only until graphos-tokens ships
- `elohim-app-frontend-gospel` §Chrome & cross-bundle composition rails — the shell side (protocol-omni trust surface, ThemeService twin, EPR-native nav)
- `lamad-bundle-gospel` §EPR-app bundle rails — the bundle-consumer side (the worked example)
- `genesis/data/timeline/backlog/bundle-styling-token-contract.md` — the canonical token artifact + runbook §4.X follow-up (entity docs stay path-cites by convention)

## Library boundary

`app/elohim-elements/<pillar>/` writes the primitive. `app/elohim-library/projects/graphos/src/default/` writes the Library A story (blank-slate + capability matrix). `app/elohim-library/projects/graphos/src/designed/` writes the Library B story (brand-bound). The three are co-authored:

- `component-architect` writes the primitive + Library A story
- `graphos-designer` reads Library A, writes Library B
- Neither agent crosses

See `app/elohim-library/CLAUDE.md` for the Library A / Library B discipline.

---

## Cross-references

- Operational structure: `app/elohim-elements/README.md`
- Pattern library discipline: `app/elohim-library/CLAUDE.md`
- Capability Profile spec: `genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md`
- Substrate principle: `genesis/docs/architecture/stewardship-over-sovereignty.md`
- SDK boundary canon: `genesis/docs/architecture/elohim-sdk.md`
- Pillar-bundle-split runbook (includes §6.14 backend-migration anti-pattern): `genesis/docs/architecture/pillar-bundle-split-runbook.md`
- Brand design spec: `genesis/graphos/elohim-protocol-design-spec.md`
- Protocol vocabulary register: `genesis/graphos/vocabulary.md`

When in doubt about scope: this file is gospel for "what is a Lit element allowed to do?"; the Capability Profile spec is gospel for "what does the element promise to render?"; §6.14 of the pillar-bundle-split runbook is gospel for "what to do when I find orchestration that shouldn't be client-side."
