---
name: graphos-dead-binding-classes
description: "Six mechanical causes of \"theming theater\" in graphos/elohim-elements — bindings byte-correct in source but dead at runtime; only renders catch them"
metadata: 
  node_type: memory
  type: project
  originSessionId: 865f6e0d-8432-40e2-a8a2-350e7d2e7e01
---

2026-06-11 frontend-eyes sprint found SIX distinct mechanical causes of dead/ghost token bindings across Library B (every one invisible to source review, all caught by rendering):

1. **Ghost names** — story binds `--elohim-reaction-count-color`, primitive consumes `-count-fg`. Cross-check designed bindings against the primitive's `@cssprop` JSDoc AND actual `var()` consumption.
2. **Border shorthand→color-slot IACVT** — story binds `1px solid X` into a var consumed as `border: 1px solid var(...)` → `1px solid 1px solid X` → invalid-at-computed-value-time → border silently NONE. Convention now: primitives consume border vars as FULL shorthand and document "(FULL shorthand … not a color)".
3. **Inline-style/JS hardcodes** — hex colors in JS data flowing into `style=` attributes (graduated-feedback DEFAULT_SCALES) or `color: CanvasText` in base CSS rules (epr-popover) beat every binding.
4. **Host `setProperty` clobber** — `updated()` unconditionally mirroring a default prop value onto the host inline style kills inherited custom-property overrides (skeleton radius). Mirror only explicit values.
5. **Inert kebab attributes** — Lit `@property()` without `attribute:` mapping lowercases the name; `display-name="Alice"` in stories silently ignored (navigator: every bubble read "Traveler", incl. aria-label). Default-true booleans additionally need a string-false-aware converter to be disableable from markup.
6. **Homonym fixture types across the primitive boundary** — designed `_lib` fixtures duck-typed into primitives with same-named but different-shaped interfaces (MockRubric.bloomMapping Record vs BloomMapping[]; CuratedEpr.title vs .label) → runtime throw or empty render. Adapt at the render seam; story files skip typecheck so tsc won't save you.

**Why:** stories aren't typechecked in the storybook build and CSS custom properties fail silently — the only reliable gate is `pnpm graphos sheet/story` + Reading the shot ([[feedback-frontend-review-eyes-first]]).

**How to apply:** when authoring/reviewing any designed story or element cssprop change, grep both sides of every binding, prefer full-shorthand border vars, and close with a local-storybook render. Also: a `*/` inside `--token-*/...` text in a block comment terminates the comment and 500s the storybook dev server.
