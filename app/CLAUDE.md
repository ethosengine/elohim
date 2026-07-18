# app/ — EPR Apps (the client surfaces of the protocol)

Each directory here that ships to users is an **EPR app**: an Angular workspace whose build
artifacts are content-addressed blobs on an EPR ContentNode, projected by any doorway or
peer runtime. The framework is a plug-in, not the architecture — Angular is the current
client; Sophia is React; the serving contract below is framework-neutral (`RenderSpec`
names the driver; `elohim-render::Renderer` is the trait a runtime drives).

| Surface | What it is |
|---------|-----------|
| `elohim-app/` | Full pillar SPA + the `elohim-host-landing` EPR (landing at `/`) |
| `lamad/` | Lamad pillar SPA — the `lamad-spa` EPR (served at `/lamad/...`) |
| `elohim-elements/`, `elohim-library/` | Lit protocol elements + Angular libraries/pattern libs (not EPR apps) |
| `imagodei-portal/` | Identity portal surface |
| `scripts/` | Cross-app conformance rails (see below) |

## What makes an app an EPR app

1. **A ContentNode** (e.g. `lamad-spa`) whose `blobHash` is the zipped browser bundle and —
   when the app is SSR-capable — whose `serverBlobHash` is the zipped server bundle. The
   root `Jenkinsfile` `stageAndVerifyAllBundles` bundle list is the declarative registry:
   one `[distDir, slug]` line per bundle, `kind: "server"` for the SSR one.
2. **A declared route surface** — `src/app/generated/route-claims.ts` (codegen). Doorway
   projections bind to claims; links are MINTED (`eprToRoute`/claims), never literal.
3. **The conformance rails** (wired into each app's `lint` script, `app/scripts/`):
   - `lint-route-literals.mjs` — no literal route minting.
   - `lint-ssr-entry.mjs` — the SSR-entry contract (below), checked at authoring time.

## Adding SSR to an EPR app (four declarative touches)

The runtime (elohim-render, consumed by doorway AND peer-native runtimes) drives the server
bundle via `mod.renderApplication(mod.default, { url })` and splices the render into the
browser shell selector-agnostically — the root tag is derived from the rendered document,
so no per-app code exists anywhere in the serving path. Node built-ins are shimmed by the
runtime (no postbuild shimming). To make an app SSR-capable:

1. **Deps**: add `@angular/platform-server` + `@angular/ssr` (same pins as elohim-app).
2. **Entries**: `src/app/app.config.server.ts` + `src/main.server.ts` — copy the pattern
   from `app/elohim-app/src/` (the canonical exemplar; ~30 lines each). The `SSR_DOCUMENT`
   template must carry YOUR app's root selector; never import `…/register` side effects in
   the server entry. `lint-ssr-entry.mjs` enforces both.
3. **Build**: `angular.json` build options — `"server"`, `"ssr": {"entry": …}`,
   `"prerender": false`; add `src/main.server.ts` to `tsconfig.app.json` `files`. One
   `ng build` then emits `dist/<app>/browser` AND `dist/<app>/server`.
4. **Seed + serve**: add the server-dist line to the root Jenkinsfile bundle list
   (`kind: "server"` → PATCHes `serverBlobHash`), and add the app slug to the doorway
   manifests' `SSR_BUNDLE_SLUGS` env (`genesis/orchestrator/manifests/doorway/*.yaml`).
   The doorway `RendererRegistry` materializes each listed slug's server bundle from the
   substrate at boot; an unseeded slug is skipped with a warn and the app stays CSR
   (`x-ssr-skipped: renderer-app-mismatch`) until the blob ships.

Verify locally before shipping — run the REAL production render against your built bundle:

```bash
cd elohim/elohim-render
RUSTFLAGS="" cargo run --example render_url -- ../../app/<app>/dist/<app>/server/main.server.mjs /<route>
RUSTFLAGS="" cargo run --example compose_check -- <rendered.html> <shell index.html>
```

A2o contract: `genesis/a2o/features/ssr/compose-serves-the-projected-app.feature`.

## Build gotchas

- **Never `pnpm install --filter <app>`** — the workspace is `shamefully-hoist=true`; a
  filtered install prunes sibling apps' hoisted deps (lamad's build reaches
  `../elohim-app` sources via tsconfig aliases). Install from the repo root.
- Declare every import in the app's own `package.json` — resolving through another app's
  hoist works until it doesn't (see `app/lamad/.epr-meta`).
