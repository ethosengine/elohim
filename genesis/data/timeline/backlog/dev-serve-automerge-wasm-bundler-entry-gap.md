---
id: "backlog-dev-serve-automerge-wasm-bundler-entry-gap"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "automerge base64 alias works in prod build but NOT in `ng serve` dev — dev pre-bundles the wasm-bindgen bundler entry → automerge_wasm_bg.wasm 500 (doc-sync broken under pnpm start)"
slug: "dev-serve-automerge-wasm-bundler-entry-gap"
written: "2026-06-27"
author: "G7 browser-leg look-rail proof (automerge content-sync plane sprint)"
status: "backlog"
priority: "medium"
jobs: [elohim]
---

## The gap (caught by the look rail, 2026-06-27)

The frontend automerge fix (G6) is a tsconfig path alias
`@automerge/automerge → .../dist/mjs/entrypoints/fullfat_base64.js` (the base64-inlined wasm
entry) in `app/elohim-app/tsconfig.json` (and the elohim-service library tsconfig). It works in
the **production build** (`ng build` — automerge inlines as base64 in a lazy chunk; verified at
runtime: `/dev/doc-sync` renders, zero httpErrors, no `automerge_wasm_bg` fetch).

**It does NOT take effect under `ng serve` (dev / `pnpm start`).** The Angular esbuild dev server
**pre-bundles dependencies via the package's `exports`/`browser` condition**, which resolves
automerge to the wasm-bindgen **bundler** entry (`...wasm_bindgen_output/bundler/automerge_wasm.js`,
whose first line is `import * as wasm from "./automerge_wasm_bg.wasm"`). The dev server then 500s
serving `automerge_wasm_bg.wasm?import`. Observed via `pnpm look`:
`httpErrors: [{status:500, url: .../automerge/dist/mjs/wasm_bindgen_output/bundler/automerge_wasm_bg.wasm?import}]`,
`pageErrors: []` (no crash — the wasm just never loads, so doc-sync silently can't work in dev).

Impact: anyone developing the doc-sync feature locally via `pnpm start` hits a non-functional
automerge (doc-sync stuck "pending"). Production / deployed app is unaffected.

## Fix options (to evaluate when picked)

1. Force the dev pre-bundler to the base64 entry too — Angular `application` builder
   `optimization`/`externalDependencies` or a `vite`/esbuild `resolve.alias` for the dev server
   (tsconfig `paths` alone is not honored by dev dependency pre-bundling for a node_modules pkg).
2. Add `@automerge/automerge` to the dev server's `prebundle.exclude` (Angular
   `"externalDependencies"`) so it's resolved through the tsconfig alias path instead of pre-bundled.
3. Configure the dev server to actually SERVE the bundler `.wasm` (so the bundler entry works in dev)
   — keeps the bundler entry, fixes the 500.
4. Make elohim-app zoneless (the only path where Angular allows wasm/ES-module integration natively)
   — biggest change; the G6 smoke-test already noted this as the refactor-grade alternative.

## Notes
- The look-rail harness `/dev/doc-sync` (committed in `1b70c0532`) is the reproduction + future
  regression surface. The browser leg is PROVEN in the production build (served `dist` + a
  wire-faithful fake `/sync` → converged doc rendered); this gap is dev-serve-only.
- Plan: `genesis/docs/superpowers/plans/2026-06-27-automerge-content-sync-plane-lighting-plan.md`
  (Task G7). Domain D5. Effort: S–M depending on option.
