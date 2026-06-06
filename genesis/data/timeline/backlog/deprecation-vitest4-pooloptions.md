---
id: "backlog-deprecation-vitest4-pooloptions"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Vitest 4 removed test.poolOptions — migrate to top-level pool worker options"
slug: "deprecation-vitest4-pooloptions"
written: "2026-06-06"
author: "deprecation-triage"
status: "stable"
priority: "low"
deprecation_status: fixed
class: process-meta
severity: low
fingerprints: [efbd9ab8fb65]
relatedNodeIds: []
tags: [deprecation, vitest, poolOptions, maxWorkers, process-meta]
cites:
  - https://vitest.dev/guide/migration#pool-rework
  - app/elohim-library/vite.config.ts
---

# Vitest 4 — `test.poolOptions` removed

## What is deprecated

The deprecation-sentinel captured (fp `efbd9ab8fb65`, 2026-06-06) on
`pnpm exec vitest run --config vite.config.ts` in `app/elohim-library`:

> DEPRECATED  `test.poolOptions` was removed in Vitest 4. All previous
> `poolOptions` are now top-level options. Please, refer to the migration
> guide: https://vitest.dev/guide/migration#pool-rework

Vitest 4 collapsed the per-pool nested option object into flat top-level
`test.*` options. `poolOptions.threads.maxThreads` and
`poolOptions.forks.maxForks` are both now the single top-level `maxWorkers`
(`minWorkers` for the floor); `isolate`, `vmMemoryLimit`, and `execArgv`
similarly moved up. `pool: 'forks'` (the pool *selector*) stays valid.

## Usage inventory

Monorepo-wide scope pass (`grep -rn "poolOptions"`, all `vite.config.*` /
`vitest.config.*` / `vitest.workspace.*` enumerated): exactly **one** config
carried the deprecated key.

- `app/elohim-library/vite.config.ts:22` — `poolOptions: { forks: { maxForks: 8 } }`

No other config uses `poolOptions`. The 16 vite/vitest configs split by major:
the six on `vitest@4.0.18` (elohim-app, elohim-library, lamad, doorway-app,
genesis/seeder, elohim-service) are the only ones where the key would warn, and
only elohim-library declared it. The remaining configs still resolve
`vitest@3.2.4` (epr-ts, storage-client-ts, elohim-agent-sdk, etc.), where
`poolOptions` is still the valid v3 shape — they are NOT in scope and must not be
touched until those projects also move to v4. One concern, one config; the
fingerprint maps 1:1.

## Migration path

Per the guide (#pool-rework) and the installed `vitest@4.0.18` config type
(`config.d.ts`: `maxWorkers: number`, `pool: string`, no `poolOptions`):

```diff
   pool: 'forks',
-  poolOptions: {
-    forks: {
-      maxForks: 8,
-    },
-  },
+  maxWorkers: 8,
```

The `forks` pool is preserved; `maxForks: 8` becomes top-level `maxWorkers: 8`
(semantics-preserving — the same 8-worker ceiling).

## Current decision

**Fixed.** Bounded one-line config migration applied to
`app/elohim-library/vite.config.ts` (the only in-scope config). No dependency
version change, no major upgrade, single file — well inside the background-agent
bound.

## Verification

Re-ran scoped suites from `app/elohim-library` after the edit
(`vitest@4.0.18`):

- [x] `pnpm exec vitest run --config vite.config.ts resilience-snapshot.component`
  → **16/16 passed**, and the `DEPRECATED ... test.poolOptions` banner is
  **gone** (grep for `DEPRECATED.*poolOptions` on the run output returns `0`).
  Pre-fix the same suite was 16/16 green *with* the banner.
- [x] `pnpm exec vitest run --config vite.config.ts distribution`
  → **13/13 passed** (2 files), no deprecation banner.

Verified 2026-06-06.
