---
title: Native Rust Chrome Omnibar — Phase 2 Implementation Plan
id: native-chrome-omnibar-plan
status: Draft
class: protocol-canonical
domain: D8
topic: [ssr, doorway, elohim-render, omnibar, chrome, composition, splice, epr-theme, progressive-enhancement, angular-migration, implementation-plan]
informed-by:
  - genesis/docs/superpowers/specs/2026-06-26-native-rust-epr-shell-ssr-design.md
cites:
  - native-rust-epr-shell-ssr-design | The design spec this plan implements (Phase 2: omnibar as native runtime chrome composed around the V8 body; §4.1 composition, §4.3 theme, §4.4 PE, §4.5 migration) | path: genesis/docs/superpowers/specs/2026-06-26-native-rust-epr-shell-ssr-design.md
requires_env: [household-nodes]
---

# Native Rust Chrome Omnibar — Phase 2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Lift the EPR omnibar out of the Angular app bundle into **native Rust runtime chrome**, composed (spliced) around the still-V8-rendered Angular body, and themed from the wrapped EPR's declared tokens — so the omnibar is identical across hosts/bundles, server-painted (not post-hydration), and immune to bundle staleness.

**Architecture:** V8 `AngularRenderer.render()` emits a **full HTML document** (`renderApplication()`). A new `ComposingRenderer` in `elohim-render` wraps the `AngularRenderer`: it fetches the EPR content node (for `metadata.theme`, the served build marker, title/description), renders the omnibar markup + a scoped themed `<style>` natively in Rust, and **splices** them into the V8 document (style into `<head>`, omnibar after the `<body>` open tag, the enhance `<script>` before `</body>`). The doorway routes its SSR dispatch through `ComposingRenderer`. A small content-addressed `omni-enhance.js` (served at `/chrome/omni-enhance.{hash}.js`) adds expand/toggle/copy/theme-persist behavior. `app-protocol-omni` is deleted from the Angular bundle.

**Tech Stack:** Rust (elohim-render composition + native templating; doorway dispatch + a tiny static route), a hand-written ~3-5KB vanilla `omni-enhance.js`, Angular 19 (deletion + shell-padding move), JSON seed.

## Global Constraints

- **Composition is a string SPLICE, not a full HTML parse.** Angular's SSR output is deterministic; splice on anchors — insert `<style>…</style>` immediately before `</head>`, the omnibar markup immediately after the `<body…>` open tag, and `<script>`s immediately before `</body>`. If an anchor is missing, return the V8 HTML **unmodified** (degrade to no-chrome with a `warn!`) — never panic, never corrupt the document. Do NOT add an HTML-parser crate dependency.
- **Themed by the EPR, base-palette fallback.** The `<style>` binds CSS custom properties from the content node's `metadata.theme` (colorScheme + tokens, added Phase 1). Absent ⇒ the base palette (the exact RGBA from today's `protocol-omni.component.css`, lifted verbatim). The omnibar markup references only `var(--omni-*, <fallback>)`.
- **Server-render the static bar; defer the dynamic detail to JS.** The native chrome server-renders the themed collapsed chip + the bar with the build marker + EPR id. The **lazy resilience panel** (today fetched on expand) and any nav-context-derived prev/next stay **client-enhanced** in `omni-enhance.js` (MVP — keeps the composition layer to ONE content-node fetch). Note this scoping in the spec's terms (§4.4).
- **RUSTFLAGS="" for doorway + elohim-render** native builds. Plain `cargo test` (no nextest). `CARGO_TARGET_DIR` pool slots; if the pool path ENOENTs (known `/projects`-volume quirk) use a `/tmp` target dir.
- **Doorway route discipline:** the `/chrome/*` static route is doorway-specific runtime chrome (like `bootstrap`/`signal`) — a match arm ABOVE the wildcard registry arm, documented as an explicit exception (`doorway-service/src/server/CLAUDE.md`). It is NOT a per-domain proxy.
- **Commit-only**, EXPLICIT-PATHSPEC commits (shared worktree carries another session's staged files; never `git add -A`/no-pathspec; `git show --stat HEAD` after each commit to prove scope).
- **Phase 1 landed:** the content node carries `metadata.theme` (schema field exists) and `serverBlobHash`; the `-ssr` row is gone. Build on that.
- The Angular deletion (T6) and the native chrome (T2–T4) must be **coherent at merge** (the served SSR HTML references the matching browser bundle, so there is no double-omnibar window). They land together.

---

### Task 1: Seed `metadata.theme` for the landing EPR + the base palette

**Files:**
- Modify: `genesis/data/lamad/content/elohim-host-landing.json` (add `metadata.theme`)
- Create: a base-palette constant in `elohim/elohim-render/src/` (e.g. `chrome/theme.rs`)
- Test: a schema-contract / fixture-validity assertion that `metadata.theme` round-trips

**Interfaces:**
- Produces: an EPR node carrying `metadata.theme = { colorScheme: "auto"|"light"|"dark", tokens: { bg, fg, muted, border, accent, shadow, envRing } }`. Absent ⇒ base palette.

- [ ] **Step 1:** Add `metadata.theme` to `elohim-host-landing.json` using the EXACT current omnibar light-palette RGBA (from `protocol-omni.component.css:20-27`) as the seeded values, `colorScheme: "auto"`.
- [ ] **Step 2:** Define `BASE_PALETTE` (the same light + dark RGBA, from `protocol-omni.component.css` `:host` + the `prefers-color-scheme: dark` block) as a Rust constant the renderer falls back to.
- [ ] **Step 3:** Add a fixture test asserting the theme shape parses and the base-palette fallback is used when absent. `python3 -m json.tool` the seed.
- [ ] **Step 4:** Commit (explicit paths): `feat(chrome): seed metadata.theme for landing EPR + base palette (omnibar T1)`.

### Task 2: Native omnibar chrome renderer (Rust)

**Files:**
- Create: `elohim/elohim-render/src/chrome/omnibar.rs` (+ `chrome/mod.rs`)
- Test: golden-HTML unit tests in the same module

**Interfaces:**
- Consumes: a `ChromeInput { slug, title, description, build_marker, theme: Option<Theme>, color_scheme }` (build the struct from the content node).
- Produces: `render_omnibar_style(theme) -> String` (the scoped `<style>` binding `--omni-*` custom props, incl. the `colorScheme`/`prefers-color-scheme` rules) and `render_omnibar_markup(input) -> String` (the collapsed chip + the bar: EPR id chip, build marker, account/theme-toggle affordances, a neutral resilience glyph, the expand/collapse controls). Markup uses only `var(--omni-*)`; behavior hooks are `data-omni-*` attributes (wired by `omni-enhance.js`).

- [ ] **Step 1:** Read `protocol-omni.component.{html,css,ts}`. Port the markup structure (collapsed chip + expanded toolbar) to a Rust string template, and the CSS (custom-property surface + light/dark) to `render_omnibar_style`. Replace Angular bindings with: server-known values (id, marker) rendered inline; behavior (expand/toggle/copy) as `data-omni-action="…"` hooks; the lazy resilience panel as a neutral glyph placeholder with `data-omni-resilience-slug` for JS. HTML-escape all interpolated values.
- [ ] **Step 2:** Golden-HTML tests: the style output for (theme present, theme absent→base, colorScheme light/dark/auto); the markup output for (with/without marker). Assert the `var(--omni-*)` surface + no raw RGBA in markup + escaped interpolation.
- [ ] **Step 3:** `RUSTFLAGS="" cargo test --lib chrome:: && cargo clippy --lib -- -D warnings && cargo fmt --check` (elohim-render). Expected: green.
- [ ] **Step 4:** Commit (explicit paths): `feat(render): native omnibar chrome renderer (markup + themed style) (omnibar T2)`.

### Task 3: `ComposingRenderer` — fetch theme + splice chrome into the V8 document

**Files:**
- Create: `elohim/elohim-render/src/composition.rs`
- Modify: `elohim/elohim-render/src/lib.rs` (export `ComposingRenderer`)
- Test: composition golden tests (chrome + stub body) + the splice-fallback test

**Interfaces:**
- Consumes: the existing `AngularRenderer` (V8) + a content-node fetch (reuse the `BundleSource`/`DataFetcher` surface or add a small `ContentFetcher`) + T2's chrome renderer.
- Produces: `ComposingRenderer` implementing `Renderer`: render via V8 → fetch the content node for `slug` → build `ChromeInput` → splice `<style>` before `</head>`, omnibar after `<body…>`, `<script src="/chrome/omni-enhance.{hash}.js">` + the browser-bundle hydration script position-check before `</body>` → return composed HTML.

- [ ] **Step 1:** Write the failing splice test: given a stub full document (`<!DOCTYPE html><html><head><title>x</title></head><body><app-root></app-root></body></html>`) and a `ChromeInput`, assert the composed output has the `<style>` before `</head>`, the omnibar markup after `<body>` and before `<app-root>`, and the enhance `<script>` before `</body>`.
- [ ] **Step 2:** Write the anchor-missing fallback test: a body with no `</head>` (or no `<body>`) returns the input HTML UNMODIFIED (degrade, no panic).
- [ ] **Step 3:** Implement the splice (string anchors, no HTML parser). Implement `ComposingRenderer::render`: call the inner `AngularRenderer`, fetch the content node (`metadata.theme`, title, description, served `blobHash` marker — Option A, one fetch via the fetcher), build `ChromeInput`, splice. On content-node fetch failure → splice with the base palette + no marker (still themed, never blocks the render).
- [ ] **Step 4:** `RUSTFLAGS="" cargo test --lib composition:: && cargo clippy --lib -- -D warnings && cargo fmt --check`. Expected: green.
- [ ] **Step 5:** Commit (explicit paths): `feat(render): ComposingRenderer splices native chrome around the V8 body (omnibar T3)`.

### Task 4: Route the doorway SSR dispatch through `ComposingRenderer`

**Files:**
- Modify: `doorway/doorway-service/src/server/http.rs` (`init_renderer` ~363-438 builds a `ComposingRenderer` wrapping the `AngularRenderer`; the SSR dispatch arm ~3629 is unchanged since it calls `renderer.render`)
- Modify: `doorway/doorway-service/src/ssr.rs` if a `ContentFetcher` is needed for the composition fetch (reuse `DoorwayBundleSource`'s `fetch_content_body`)

**Interfaces:**
- Consumes: T3. Produces: `AppState.renderer` is now the `ComposingRenderer`; every `angular-ssr` route gets native chrome with zero new dispatch wiring.

- [ ] **Step 1:** In `init_renderer`, after constructing `AngularRenderer`, wrap it in `ComposingRenderer` (supplying the content-fetch surface + the `/chrome/omni-enhance.{hash}.js` URL from T5). Return the composer as `Arc<dyn Renderer>`. Preserve the `None` → CSR-fallback safety.
- [ ] **Step 2:** `RUSTFLAGS="" cargo build --release && cargo test --lib --bins ssr && cargo clippy -- -D warnings && cargo fmt --check` (doorway). Expected: green.
- [ ] **Step 3:** Commit (explicit paths): `feat(doorway): route SSR dispatch through ComposingRenderer (omnibar T4)`.

### Task 5: `omni-enhance.js` + the `/chrome/` static route

**Files:**
- Create: `elohim/elohim-render/src/chrome/omni-enhance.js` (hand-written vanilla JS, ~3-5KB)
- Create: `doorway/doorway-service/src/routes/chrome.rs` (+ export in `routes/mod.rs`)
- Modify: `doorway/doorway-service/src/server/http.rs` (a `/chrome/` match arm ABOVE the wildcard, documented exception)
- Test: a doorway route test (200 + correct content-type + the hash in the path)

**Interfaces:**
- Produces: `GET /chrome/omni-enhance.{sha256}.js` → the script with `Content-Type: text/javascript` + long cache headers (content-addressed = immutable). The script's hash is computed at build/const time and exposed to T2/T4 so the `<script src>` matches.

- [ ] **Step 1:** Write `omni-enhance.js`: wire `data-omni-action="toggle"` (expand/collapse class), `="copy"` (clipboard + aria-live feedback), theme toggle (`localStorage['elohim-theme']` ↔ `html[data-theme]`, dispatch `elohim-theme-changed`), and the lazy resilience fetch (`data-omni-resilience-slug` → GET nav-context/snapshot on first expand, populate). Plain `<a href>` nav needs no JS.
- [ ] **Step 2:** Bake the script into the binary (`include_str!`) and compute its `sha256` as a const (a `build.rs` or a `const` + a test asserting the hash matches the bytes). Export the content-addressed path so T2/T4 reference the same hash.
- [ ] **Step 3:** Add `routes::handle_chrome_asset` + the `/chrome/` dispatch arm (above the wildcard; comment: runtime chrome, not a proxy). Serve the script, immutable cache headers.
- [ ] **Step 4:** Route test (200, content-type, hash path). `RUSTFLAGS="" cargo test --lib --bins chrome && cargo clippy -- -D warnings && cargo fmt --check`. Expected: green.
- [ ] **Step 5:** Commit (explicit paths): `feat(doorway): omni-enhance.js + /chrome static route (omnibar T5)`.

### Task 6: Delete the omnibar from the Angular bundle

**Files:**
- Delete: `app/elohim-app/src/app/elohim/components/protocol-omni/` (+ `protocol-omnibar/` if a distinct component — VERIFY first)
- Modify: `app/elohim-app/src/app/app.component.{ts,html,css}` (remove the `<app-protocol-omni>` use, the `ProtocolOmniComponent` import, the `host` `with-omni` binding; the top padding moves to the runtime shell, so remove `.with-omni` and ensure `body`/`app-root` has no omnibar reservation)
- Modify: `app/elohim-app/src/app/elohim/components/index.ts` (drop the export); `serving-context.model.ts` + `config.service.ts` (remove ONLY if the omnibar was the sole consumer — verify with grep)
- Modify/Delete: the omnibar `.spec.ts` + app.component spec assertions

**Interfaces:**
- Consumes: nothing (independent surface). Produces: an Angular bundle with no omnibar; the runtime shell owns the top spacing.

- [ ] **Step 1:** `grep -rn "ProtocolOmni\|protocol-omni\|with-omni\|servingContext\|ServingContext\|app-protocol-omnibar" app/elohim-app/src` to map every reference. Determine whether `protocol-omnibar`, `serving-context.model`, `config.service` `gitHash` usage are omnibar-only (delete) or shared (keep). Record the decision.
- [ ] **Step 2:** Delete the omnibar component(s) + remove the `app.component` usage/import/host-binding; move the top-padding responsibility to the shell (the native chrome occupies the top, so the Angular body must not also reserve `with-omni` space — confirm the shell's `<body class>` / padding handles it).
- [ ] **Step 3:** Remove dead barrels/models/specs flagged in Step 1.
- [ ] **Step 4:** `pnpm --filter elohim-app build` (or the repo's app build) — bundle compiles, shrinks; `pnpm --filter elohim-app test` for the touched specs; `pnpm --filter elohim-app lint`. Expected: green.
- [ ] **Step 5:** Commit (explicit paths): `feat(app): remove omnibar from Angular bundle — now runtime chrome (omnibar T6)`.

### Task 7: Cleanup sweep + whole-branch review

- [ ] **Step 1:** `grep -rn "protocol-omni\|ProtocolOmni" app/elohim-app/src` → zero live references (only historical/spec mentions remain).
- [ ] **Step 2:** Confirm the end-to-end render contract: the composed HTML has the themed `<style>` + omnibar + the `/chrome/omni-enhance.{hash}.js` reference whose hash matches the served route; absent `metadata.theme` ⇒ base palette (not blank); content-node fetch failure ⇒ base-palette chrome, never a blocked render.
- [ ] **Step 3:** Whole-branch review (most capable model): the splice correctness (anchor injection + fallback), HTML-escaping of interpolated values (XSS surface), the `/chrome` route exception justification, the Angular deletion completeness, and triage of any Minor findings.
- [ ] **Step 4:** Progress-ledger update. Touched-tree gates: elohim-render + doorway (`RUSTFLAGS="" … clippy -D warnings && fmt --check`); elohim-app (build + lint).

## Done

- The doorway serves SSR HTML with a **natively-rendered, themed omnibar** spliced around the V8 Angular body; `app-protocol-omni` is gone from the bundle.
- `metadata.theme` drives the palette; absent ⇒ base palette; fetch/anchor failure ⇒ graceful degrade (base palette / unmodified doc), never a panic or blocked render.
- Interpolated values are HTML-escaped (no XSS via title/marker/theme).
- elohim-render + doorway + elohim-app gates green.
- **Post-merge / post-deploy** (not a local gate): the a2o visual regression — the omnibar themed correctly across light/dark on **both** alpha and elohim.host (the originating defect) — verified via `pnpm look` + the theme probe once `dev` deploys.

## DELIVERY REVISED (2026-06-26): runtime-served client element (supersedes the SSR-splice tasks)

Tauri is CSR, so the omnibar is delivered as a runtime-served self-contained vanilla web-component ELEMENT (client-rendered in browser, /deliver, and Tauri identically). Supersedes old T2-render / T3-splice / T4-dispatch.

- **B-T1 (the element)** — self-mounts; acquires EPR context (inline `<script id=elohim-omni-context>` or fetch); renders the rich omnibar (ported from `omnibar.rs` design); themes from `metadata.theme` (base-palette fallback); folds in `omni-enhance.js` behavior; served content-addressed `/chrome/omni-element.{hash}.js`. [DISPATCHED]
- **B-T2 (sidecar serving)** — add the `/chrome` route to the elohim-storage sidecar (the device peer runtime serves the element to its local Tauri webview + connecting peers); mirror doorway `routes/chrome.rs` + the `is_service_path` guard; gate with the peer render-capability (runtime-layer enable/disable).
- **B-T3 (references)** — doorway injects the inline EPR context + the element `<script>` into SSR'd HTML (replaces the splice); the Tauri SPA `app/elohim-app/src/index.html` references the local sidecar's `/chrome` element.
- **B-T7 (review)** — whole-branch review (JS XSS escaping, context-acquisition, the `/chrome` exception, dual-serving); a2o visual regression post-deploy.

REUSE: T1 (theme), T5 (/chrome route + content-addressing + behavior JS), T6 (both omnibars deleted; /deliver resolved by B).
DEFER: old T2 Rust render = the element's reference design + the future SSR-first-paint runtime toggle.
