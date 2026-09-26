---
id: "backlog-deprecation-analogjs-vite-optimizedeps-esbuildoptions"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "@analogjs/vite-plugin-angular emits optimizeDeps.esbuildOptions because its vite peer resolves to 7.3.1 while vitest 4.1 runs Vite 8"
slug: "deprecation-analogjs-vite-optimizedeps-esbuildoptions"
written: "2026-09-24"
author: "deprecation-triage"
status: "backlog"
priority: "low"
deprecation_status: blocked
severity: low
fingerprints: ["612e34199acb", "8a122f0ee621"]
relatedNodeIds: []
tags: [deprecation, vite, vite-8, rolldown, analogjs, vite-plugin-angular, vitest, lamad, elohim-app, elohim-library, peer-dependency]
cites:
  - app/lamad/vite.config.ts
  - app/lamad/package.json
  - app/elohim-app/package.json
  - app/elohim-library/package.json
  - pnpm-workspace.yaml
  - pnpm-lock.yaml
  - https://vite.dev/guide/migration
---

## What is deprecated

```
[vite] warning: `optimizeDeps.esbuildOptions` option was specified by "@analogjs/vite-plugin-angular" plugin. This option is deprecated, please use `optimizeDeps.rolldownOptions` instead.
```

This warning has been captured twice so far:

- `612e34199acb`, from `pnpm exec vitest run --config vite.config.ts` in
  `app/lamad`.
- `8a122f0ee621` (2026-09-25), the same text behind Vite's time-of-day
  logger prefix (`1:34:35 PM [vite] warning: …`). That prefix is part of the
  line the sentinel hashes, so every timestamped emission gets a new
  fingerprint. This is recorded as Class 13 in
  `deprecation-sentinel-redundant-capture-surfaces.md`.

## Usage inventory

None of our own configs set `optimizeDeps.esbuildOptions`. The plugin sets it
at `@analogjs/vite-plugin-angular@2.6.4` `src/lib/utils/plugin-config.js:90`:

```
...(vite.rolldownVersion ? { rolldownOptions } : { esbuildOptions }),
```

**The cause is peer-resolution skew in our lockfile, not an upstream bug.**
(This corrects the 2026-09-24 reading of the same entry.) Here is how the
versions resolve, checked with `require.resolve` from `app/lamad` on
2026-09-26:

| Consumer | Resolves `vite` to |
|---|---|
| `vitest@4.1.11`, which runs the tests | **8.1.5** (Rolldown build, which has `rolldownVersion`) |
| `@analogjs/vite-plugin-angular@2.6.4`, which calls `vite.rolldownVersion` | **7.3.1** (no `rolldownVersion`) |

The plugin checks the Vite it imports (7.3.1), sees no `rolldownVersion`, and
emits the legacy key. Vite 8, which actually runs, then flags that key as
deprecated. The plugin's peer range already allows `vite ^6 || ^7 || ^8`.

The skew comes from our own declarations:

- `app/elohim-app/package.json`: `"vite": "^7.3.1"` next to `"vitest": "^4.1.0"`.
- `app/elohim-library/package.json`: `"vite": "^7.3.1"` next to `"vitest": "^4.1.0"`.
- `app/lamad/package.json` declares no `vite`, so its plugin instance takes
  the 7.x peer that the workspace hoists.

The lockfile holds six `@analogjs/vite-plugin-angular@2.6.4` peer variants,
alongside `vite@6.4.1`, `7.3.1` and `8.1.5`.

## Migration path

Have the plugin resolve the same Vite that vitest runs:

1. Bump `"vite"` from `^7.3.1` to `^8.1.5` in `app/elohim-app/package.json`
   and `app/elohim-library/package.json`. Add `"vite": "^8.1.5"` as a
   devDependency in `app/lamad/package.json`, so the plugin peer in each
   workspace is 8.x.
2. Run `pnpm install`, then check that every `@analogjs/vite-plugin-angular`
   peer variant in `pnpm-lock.yaml` points at `vite@8.1.5`.
3. Check whether anything still depends on `vite@7`. The
   `@angular-devkit/build-angular@19.2.22` / `@angular/build` dev-server
   chain may pin its own `vite`. That pin is separate and can stay.
4. Verify: `pnpm exec vitest run --config vite.config.ts` in `app/lamad`,
   `app/elohim-app` and `app/elohim-library`. Each must be green with no
   `optimizeDeps.esbuildOptions` banner. Then run `just gate` for the three
   projects, plus an `ng build` in `app/elohim-app`, because Vite 8 moves
   dependency optimization to Rolldown.

## Current decision

**Blocked.** The fix is a major-version bump (Vite 7 to 8) of a first-party
devDependency in two workspaces, plus a lockfile change. Under the triage
agent's hard rule, that needs an operator-started dependency pass, not a
background agent working in a shared worktree. The plan above is ready to
run as written.

The warning is cosmetic in the meantime. The suites it comes from run and
pass. The sentinel will not dispatch again on `612e34199acb` or
`8a122f0ee621` (both ledger rows are `blocked`). A new timestamped emission
can still mint a new fingerprint until Class 13 is normalized. When that
happens, fold the fingerprint in here.

## Verification

N/A. Not fixed; blocked on the Vite 8 devDependency alignment above.
