---
id: lamad-bundle-gospel
cites:
  - elohim-elements-ui-substrate-gospel | layer rails — element/token/binding ownership this bundle consumes (never defines) | sha256:84cff1a46650cf8f | path: app/elohim-elements/CLAUDE.md
  - elohim-app-frontend-gospel | the shell twin of these rails — chrome composition + cross-bundle navigation rules | sha256:5e339d814c53974b | status: stale — target content moved on; re-verify | path: app/elohim-app/CLAUDE.md
  - genesis/data/timeline/backlog/bundle-styling-token-contract.md
---

# Lamad Reference Client Views

This directory contains the reference Angular client's view layer for the
lamad (learning) domain. Generated types, renderers, and components.

The domain vocabulary (manifest, schemas, coupling declarations) lives in
`elohim/sdk/domains/lamad/`. This directory consumes those definitions.

## Generated Types

Types in `generated/` are produced by `sdk/domains/lamad/scripts/codegen.mjs`.
Do not hand-edit — regenerate with `pnpm run lamad:codegen`.

## EPR-app bundle rails (separation of concerns)

This is an independently-served EPR-app bundle (`<base href="/lamad/">`), not a module of the shell. The bundle IMPORTS layers; it never defines them:

- **Styling**: `src/styles.scss` imports, in order: the base layer (`elohim-core/base.scss` — universal reset + a11y floor; without it the UA's `body{margin:8px}` frames the viewport), the token layer (`elohim-core/tokens.scss` — palette + `color-scheme` + `:root[data-theme]` theme reactivity), and the chrome-binding layer (`src/_chrome-binding.scss` — interim home, ONE concern: chrome `--elohim-*` cssprops → palette, every bound `*-bg` paired with a bound `*-fg`; it migrates wholesale to the graphos-tokens artifact). Never define or duplicate `--lamad-*` tokens in this bundle; never add bundle styles to the binding file.
- **Chrome**: `<elohim-page-chrome>` wraps the root (auto-installs the epr-link interceptor); `<elohim-navigator>` is the Lit element — its `(navigate)` CustomEvent must be wired (see `lamad-layout.component.ts:onNavigatorNavigate`): lamad-owned routes → this router (base-href-stripped), everything else → full doorway load.
- **Cross-bundle links**: under this base href, `routerLink="/"` resolves to lamad's OWN home — links to the landing/shell must be plain `href` (the interceptor records the nav handoff). Cross-bundle CONTENT targets mint the universal `/epr/{id}` address (template anchors: plain href; programmatic: `LAMAD_EPR_NAV.navigate()`); lamad claims only `contentType: 'path'` in its `BUNDLE_ROUTE_CONTEXT` (app.config). An absolute `redirectTo` in this router re-enters ITSELF (self-loop) — never use it for cross-bundle escapes (that's why the `/lamad/resource/{id}` legacy bridge is a component).

Concern routing (content-addressed — slugs resolve via this file's `cites:` frontmatter and survive moves):
- `elohim-elements-ui-substrate-gospel` §Layer rails — element/token/binding layer ownership
- `elohim-app-frontend-gospel` §Chrome & cross-bundle composition rails — the shell twin of these rails
- `genesis/data/timeline/backlog/bundle-styling-token-contract.md` — canonical token artifact (deletes the interim binding file)

## See Also

- Domain vocabulary: `elohim/sdk/domains/lamad/CLAUDE.md`
- Protocol schemas: `elohim/sdk/schemas/CLAUDE.md`
