---
id: project-elohim-app-local-build-verification-gaps
name: project_elohim_app_local_build_verification_gaps
title: elohim-app local build verification gaps
description: "In-container gates miss strictTemplates AOT errors (tsc/JIT) — verify with a direct `ng build`; buffer bundle error = install-state, not code."
metadata: 
  node_type: memory
  type: project
  originSessionId: bb6233b0-21d0-494c-8768-a211d858c47c
---

In this dev container the elohim-app quality gates can't all run locally — and the gaps hide a real CI-blocking error class:

- **`pnpm run build` fails at `prebuild`** → `build:wasm` runs `wasm-pack` (for `elohim/elohim-cache-core`), which is NOT installed in the container (`wasm-pack: command not found`). So the full AOT build never starts via the npm script.
- **ESLint can't even load its config**: `Cannot find module 'ts-api-utils'` — a pnpm strict-resolution/hoisting gap in `eslint-plugin-sonarjs` (the dep IS installed, two versions, and in the lockfile, but the plugin can't resolve it from its own location). So `pnpm run lint` is dead locally. Defer lint to CI.
- **`tsc --noEmit -p tsconfig.app.json` and vitest JIT both MISS Angular `strictTemplates` (ngtsc/AOT) template-type errors.** `strictTemplates: true` is on (`tsconfig.json`). Classic miss: indexing an interface with no index signature by a key the template widened to `string` (e.g. an inline `*ngFor="let row of [{key:'x'},…]"` widens `row.key` to `string`, so `blocks()[row.key]` is an AOT error TS7053) — fixed by moving the list to a component field typed `{ key: keyof TheInterface; … }[]`. tsc/JIT pass; only `ng build` (CI) fails.

**Verify AOT locally despite the prebuild block:** run `pnpm exec ng build --configuration development` DIRECTLY — it skips the npm `prebuild` hook (so no wasm-pack), and ngtsc does AOT template type-checking BEFORE esbuild bundling. If it fails only at the missing-wasm *module resolution* (bundle phase), your templates are AOT-clean; if it fails earlier with a template TS error, you have a real strictTemplates bug. This is the one local AOT signal when `pnpm run build` is blocked.

- **The `Could not resolve "buffer"` bundle errors are NOT a code bug and block ALL local rendering.** `@bitgo/blake2b-wasm` (`require('buf'+'fer')`) and `safe-buffer` (`require('buffer')`) — Holochain crypto deps pulled in at app bootstrap — fail esbuild's node-builtin polyfill resolution from inside the `.pnpm` virtual store. `buffer` IS installed (`app/elohim-app/node_modules/buffer`, added as a direct dep in `f1fe16e3e` for CI) and resolves via Node, but esbuild can't resolve it from the deep `.pnpm/@bitgo+blake2b-wasm` location in THIS container's install state. CI hoists/installs correctly so it builds there. **Consequence: eyes-first `/debug` render is infeasible locally** — Angular 19's dev-server (`pnpm start` / direct `ng serve`) uses the SAME esbuild pipeline, so it fails identically; `pnpm start:alpha` too. The buffer error is bundle-phase (AFTER ngtsc), so seeing ONLY buffer errors (no template TS errors) is positive confirmation your templates are AOT-clean. Don't chase it with `pnpm install` mid-branch (heavy, risks lockfile drift) — it's a documented container install-state gap; CI is the render path.

Verification spine when these gaps apply: vitest (JIT render — `TestBed.createComponent + detectChanges` exercises templates) + `tsc --noEmit` + a direct `ng build` for AOT template-check + format via `prettier --check` (works) — and let CI run the full build + eslint. Related: [[project_container_cargo_environment_quirks]] (the cargo-side `/projects` fingerprint-ENOENT quirk → use /tmp target dirs; never pipe gate exit codes).
