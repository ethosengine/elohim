---
name: elohim-ui Lit/WC pivot — design for a generation
description: Component layer pivots to Lit Web Components instead of Angular libraries; pay the framework-agnostic tax upfront so the protocol outlives any single frontend framework
type: project
originSessionId: 75ea3d40-9dd2-4e0c-ba96-dcb49c5221b5
---
Decision (2026-05-06): the elohim component library pivots to Lit-based Web Components. Original plan was Path 2 (Angular libraries via ng-packagr); user pivoted because the protocol is meant to outlive any single frontend framework, and `<sophia-question>` already validates the WC boundary in this stack.

**Why:** rejected `@angular/elements` escape hatch and Stencil because Lit is the lightest substrate (~5KB runtime), fewest compiler seams, cleanest standards-aligned story. Big design systems with multi-framework consumers (Adobe Spectrum, IBM Carbon, Microsoft Fluent, Salesforce Lightning) all went Lit or Stencil — the pattern is mature, not speculative. User's framing: "design for a generation, no shortcuts."

**How to apply:**
- The eight `app/elohim-styles/` modules are the pivot target. Likely renamed (`elohim-ui`?). Tokens stay (CSS custom properties penetrate Shadow DOM — they're the correct substrate for Lit anyway).
- Each module ships Lit components with TypeScript source, `html` template literals, Shadow DOM scoped styles, custom-elements-manifest for typing + storybook.
- elohim-app inversion: Angular consumes custom elements via `CUSTOM_ELEMENTS_SCHEMA`; component shells shrink to thin wrappers over the WC, eventually disappearing.
- Storybook: `@storybook/web-components` framework alongside the existing `@storybook/angular` in graphos.
- Don't migrate 50+ components in one sprint — prove the pattern end-to-end on one atom first, then replicate.

**Hard constraints:**
- Lit, not Stencil. Tokens cascade is sacred (`var(--*)`, never hardcoded colours). Custom-elements-manifest required for every component. Accessibility-first (ARIA, keyboard nav, focus management).
- The 50+ Angular components in `app/elohim-app/src/app/{imagodei,lamad,shefa,qahal,doorway,avodah,account,components}/` are migration sources, not lift-and-shift targets — rewrite as Lit.
