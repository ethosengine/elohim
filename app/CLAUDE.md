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
   - `lint-workspace-imports.mjs` — the cross-workspace import ratchet (below).

## Bundle seams are not domain seams

Splitting an app out of `app/elohim-app/` into its own workspace buys ONE thing: an
independently built, independently served, separately content-addressed bundle. That is a
**deployment** fact. It does not make the extracted tree a self-contained domain, and it
does not create a module boundary — because each app's `tsconfig.json` `paths` maps its
sibling `@app/<pillar>/*` aliases straight at the other workspace's private `src/app/*`.
Deep paths, both directions, no public API. Two `package.json`s, one TypeScript program.

The failure mode is that the directory *looks* like a boundary, so nobody checks that it
is one. Each individual cross-workspace import is locally reasonable; the arrow direction
and the cycle are only visible in aggregate, and nothing was aggregating. `app/lamad/` is
the lived case: it holds both the learning domain AND the cross-pillar content substrate
(`models/content-node.model`, `content-io/`, `renderers/`, `parsers/`), five pillars import
that substrate, `elohim` — the core — is one of them, and `lamad` imports back into six
elohim-app pillars from `models/`, `services/`, `interfaces/`, `utils/` and `guards/`.

So: **before extracting an app, decide what the extracted tree owns and what it consumes**,
and give each direction a named entry point. Before adding an import that crosses an
existing seam, ask whether the type belongs to that bundle's domain or in a home both
bundles consume. The rail below makes that a decision instead of an accident.

### The cross-workspace import ratchet

`lint-workspace-imports.mjs <appDir>` reads the app's own tsconfig `paths`, keeps the
`@app/*` aliases that resolve **outside** the app root, and counts every reference to them.
`app/scripts/workspace-import-baseline.json` records today's set as **declared debt** —
per-specifier counts, plus a header naming what that debt is. The rail fails on:

- a **new** cross-workspace specifier — an edge nobody declared, or
- a **higher count** on an existing one — the entanglement deepening.

Shrinking is the only allowed direction; it reports the shrink and asks you to re-baseline
(`--write-baseline`). When a direction reaches zero, delete its entry — the seam is then
real and cannot silently reopen. An app with cross-workspace aliases and **no** baseline
entry fails too: a missing baseline is an unmeasured seam, not a clean one.

The rail deliberately takes no position on which placement is correct — that is an architect
call (`genesis/data/timeline/backlog/arch-frontend-bundle-seams-backlog.md` row 1). It only
guarantees the next such drift is chosen rather than accumulated.

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
