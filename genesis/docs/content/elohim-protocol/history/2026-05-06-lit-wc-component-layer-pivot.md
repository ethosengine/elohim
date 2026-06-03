---
title: "History/ADR: The component layer is Lit Web Components, not Angular libraries"
id: lit-wc-component-layer-pivot
type: history-decision
status: Accepted
created: 2026-05-06
topic: [frontend, component-library, web-components, lit, design-system, framework-agnostic]
# A settled decision (the path-not-taken is Angular-libraries-via-ng-packagr).
canonical:
  - genesis/docs/content/elohim-protocol/architecture/2026-06-02-doorway-ssr-runtime.md   # the edge that server-renders these framework-agnostic WCs
memory_anchors:
  - project_subsume_g_f_a_via_it_just_works   # "design for a generation" — substrate must outlive its frontends
  - project_elohim_app_as_composable_view_federation
---

# History/ADR: The component layer is Lit Web Components, not Angular libraries

> **Hot-context pointer (the one sentence to remember):**
> **The protocol's component layer is framework-agnostic Lit Web Components — not Angular libraries.**
> We pay the WC tax upfront so the substrate outlives any single frontend framework. Angular *consumes*
> custom elements; it does not own the component vocabulary.

## The path not taken

The original plan (Path 2) was to ship the component library as **Angular libraries via `ng-packagr`** —
the lift-and-shift of the ~50 existing Angular components in
`app/elohim-app/src/app/{imagodei,lamad,shefa,qahal,doorway,avodah,account,components}/`. We pivoted before
building it. The framing that decided it: **"design for a generation, no shortcuts"** — a protocol meant to
outlive any one frontend cannot bind its component vocabulary to Angular's lifecycle. The
`<sophia-question>` web component already proved the WC boundary works inside this stack, so the agnostic
substrate was validated, not speculative.

**Why Lit, and why not the alternatives:**
- **Rejected `@angular/elements`** — an escape hatch that still couples authoring to Angular.
- **Rejected Stencil** — a heavier compiler seam.
- **Chose Lit** — the lightest substrate (~5KB runtime), fewest compiler seams, cleanest
  standards-aligned story. Every major multi-framework design system (Adobe Spectrum, IBM Carbon,
  Microsoft Fluent, Salesforce Lightning) converged on Lit or Stencil. The pattern is mature.

## The settled shape

- The eight `app/elohim-styles/` modules are the pivot target (likely renamed, e.g. `elohim-ui`). **Design
  tokens stay** — CSS custom properties penetrate Shadow DOM, so the `var(--*)` token cascade is already the
  correct substrate for Lit.
- Each module ships Lit components: TypeScript source, `html` template literals, Shadow-DOM-scoped styles,
  and a **custom-elements-manifest** (required for every component — it's the typing + Storybook source).
- **Angular inverts to a consumer:** it pulls custom elements in via `CUSTOM_ELEMENTS_SCHEMA`; the existing
  Angular component shells shrink to thin wrappers over the WC and eventually disappear. The ~50 Angular
  components are migration *sources* (rewrite as Lit), not lift-and-shift targets.
- **Storybook:** add the `@storybook/web-components` framework alongside the existing `@storybook/angular`
  in graphos.

## Hard constraints (do not relitigate)

- **Lit, not Stencil.** The token cascade is sacred — `var(--*)`, never hardcoded colours.
- **Custom-elements-manifest for every component.** Accessibility-first by construction: ARIA, keyboard
  nav, focus management.
- **Prove the pattern end-to-end on one atom first, then replicate.** Do not attempt the 50+ component
  migration in one sprint.

## Why this matters downstream

Framework-agnostic components are what let the [doorway SSR runtime](../architecture/2026-06-02-doorway-ssr-runtime.md)
server-render the experience at the web2 projection edge without dragging an Angular runtime along — the
WC boundary is the same seam that keeps the elohim-app a [composable view federation](../architecture/MAP.md)
rather than a single-framework monolith.
