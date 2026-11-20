---
id: project-elohim-app-local-build-verification-gaps
name: project_elohim_app_local_build_verification_gaps
title: "elohim-app/lamad local build & serve traps (umbrella)"
description: "Gates miss strictTemplates AOT (run ng build); lamad needs --serve-path /lamad/; mesh lanes need a development dist."
metadata: 
  node_type: memory
  type: project
  originSessionId: bb6233b0-21d0-494c-8768-a211d858c47c
cites:
  - app/elohim-app/package.json
  - app/elohim-app/tsconfig.json
---

In this dev container the elohim-app quality gates can't all run locally — and the gaps hide a real CI-blocking error class:

- **`pnpm run build` fails at `prebuild`** → `build:wasm` runs `wasm-pack` (for `elohim/elohim-cache-core`), which is NOT installed in the container (`wasm-pack: command not found`). So the full AOT build never starts via the npm script.
- **ESLint can't even load its config**: `Cannot find module 'ts-api-utils'` — a pnpm strict-resolution/hoisting gap in `eslint-plugin-sonarjs` (the dep IS installed, two versions, and in the lockfile, but the plugin can't resolve it from its own location). So `pnpm run lint` is dead locally. Defer lint to CI.
- **`tsc --noEmit -p tsconfig.app.json` and vitest JIT both MISS Angular `strictTemplates` (ngtsc/AOT) template-type errors.** `strictTemplates: true` is on (`tsconfig.json`). Classic miss: indexing an interface with no index signature by a key the template widened to `string` (e.g. an inline `*ngFor="let row of [{key:'x'},…]"` widens `row.key` to `string`, so `blocks()[row.key]` is an AOT error TS7053) — fixed by moving the list to a component field typed `{ key: keyof TheInterface; … }[]`. tsc/JIT pass; only `ng build` (CI) fails.

**Verify AOT locally despite the prebuild block:** run `pnpm exec ng build --configuration development` DIRECTLY — it skips the npm `prebuild` hook (so no wasm-pack), and ngtsc does AOT template type-checking BEFORE esbuild bundling. If it fails only at the missing-wasm *module resolution* (bundle phase), your templates are AOT-clean; if it fails earlier with a template TS error, you have a real strictTemplates bug. This is the one local AOT signal when `pnpm run build` is blocked.

- **The `Could not resolve "buffer"` bundle errors are NOT a code bug and block ALL local rendering.** `@bitgo/blake2b-wasm` (`require('buf'+'fer')`) and `safe-buffer` (`require('buffer')`) — Holochain crypto deps pulled in at app bootstrap — fail esbuild's node-builtin polyfill resolution from inside the `.pnpm` virtual store. `buffer` IS installed (`app/elohim-app/node_modules/buffer`, added as a direct dep in `f1fe16e3e` for CI) and resolves via Node, but esbuild can't resolve it from the deep `.pnpm/@bitgo+blake2b-wasm` location in THIS container's install state. CI hoists/installs correctly so it builds there. **Consequence: eyes-first `/debug` render is infeasible locally** — Angular 19's dev-server (`pnpm start` / direct `ng serve`) uses the SAME esbuild pipeline, so it fails identically; `pnpm start:alpha` too. The buffer error is bundle-phase (AFTER ngtsc), so seeing ONLY buffer errors (no template TS errors) is positive confirmation your templates are AOT-clean. Don't chase it with `pnpm install` mid-branch (heavy, risks lockfile drift) — it's a documented container install-state gap; CI is the render path.

Verification spine when these gaps apply: vitest (JIT render — `TestBed.createComponent + detectChanges` exercises templates) + `tsc --noEmit` + a direct `ng build` for AOT template-check + format via `prettier --check` (works) — and let CI run the full build + eslint. Related: [[project_container_cargo_environment_quirks]] (the cargo-side `/projects` fingerprint-ENOENT quirk → use /tmp target dirs; never pipe gate exit codes).

**Folded members (memorialized 2026-09-22 — detail lives in each file):**

- [[project_mesh_browser_lane_needs_dev_config_dist]] — folded 2026-09-22 (index: false); Household mesh-browser lanes need elohim-app built with --configuration development; production/alpha builds bake doorway-alpha and hosted-steward scenarios 401 there
- [[project_lamad_local_dev_serve_traps]] — folded 2026-09-22 (index: false); Rendering app/lamad locally needs --serve-path /lamad/ and DOORWAY_TARGET=localhost:8888 (mesh doorway); anonymous alpha content reads are reach-gated.

**Fresh-worktree mesh traps (2026-10-30, integration shift):** a mesh started from a worktree under `/projects/elohim/.worktrees/<name>/` cannot generate conductors — holochain panics `path must be shorter than SUN_LEN` because the Lair keystore socket path under `<worktree>/genesis/local-dev/household-dowell/conductors/…` exceeds the Unix socket limit (108 bytes); doorways/relay/mongo/portal still come up, so `just mesh status` reads UP for all but conductors and storage. Cure: `MESH_DIR=/projects/elohim/.worktrees/lm` (short). Also: preflight refuses a pool storage/doorway binary whose recorded derivation key differs from the closure at HEAD (src-mtime fallback when no record) (`just mesh build doorway|storage` after any doorway/storage change), and the prologue FATALs without `app/lamad/dist/lamad/browser` + `app/elohim-app/dist/elohim-app/browser` (copy a built dist in when the lane never loads the app).
Under a short MESH_DIR the next refusal is the hApp: a fresh worktree has no packed `elohim/holochain/dna/elohim/workdir/elohim.happ` (gitignored build output), so `hc sandbox generate` dies `AppBundleError(FfsIoError … NotFound)` after the doorways are already up — set `MESH_HAPP_PATH=/projects/elohim/elohim/holochain/dna/elohim/workdir/elohim.happ` (the main checkout's pack; honest when the line carries no DNA change) or `just build` in that DNA dir first.
