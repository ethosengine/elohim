---
id: "backlog-deprecation-content-resolver-register-all-standard-sources-retire"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Retire ContentResolverService.registerAllStandardSources() — deprecated, zero production callers, test-scaffolding only"
slug: "deprecation-content-resolver-register-all-standard-sources-retire"
written: "2026-07-30"
author: "deprecation-triage"
status: "backlog"
priority: "low"
deprecation_status: open
severity: low
fingerprints: ["8d730f4f6528"]
relatedNodeIds: []
tags: [deprecation, angular, elohim-app, lamad, content-resolver, dead-api, test-scaffolding]
cites:
  - app/elohim-app/src/app/elohim/services/content-resolver.service.ts
  - app/elohim-app/src/app/elohim/services/content-resolver.service.spec.ts
  - app/lamad/src/app/services/content-resolver.service.ts
  - genesis/data/timeline/backlog/deprecation-conductor-content-tier-retirement-strategy-seam.md
---

## What is deprecated

```
* @deprecated Use initializeForMode() instead for mode-aware source registration
```

On `ContentResolverService.registerAllStandardSources(urls?)`
(`content-resolver.service.ts:299`). It registers the fixed six-source set
(`indexeddb`, `projection`, `conductor`, `edgenode`, `dht`, `cdn`) with no
awareness of the active connection mode — the exact thing
`initializeForMode(strategy, config)` exists to replace.

## Usage inventory

Repo-wide grep (excluding `node_modules`, `coverage`, `dist`) — **zero
production callers in either workspace**:

| Site | Kind |
|---|---|
| `app/elohim-app/src/app/elohim/services/content-resolver.service.ts:299` | declaration |
| `app/lamad/src/app/services/content-resolver.service.ts:299` | declaration (mirror copy) |
| `app/elohim-app/…/content-resolver.service.spec.ts` :228, :241, :329, :387, :505, :566, :591, :671 | 8 test call sites |

The `app/lamad` copy has **no** caller at all — that workspace has no
`content-resolver.service.spec.ts` (only `data-loader.service.spec.ts`).

Production registers sources individually instead: `data-loader.service.ts`
calls `registerStandardSource('indexeddb' | 'projection' | 'conductor')`. So the
deprecation has *already fully landed in production*; only test scaffolding
keeps the deprecated API alive.

## Migration path

The `@deprecated` note points at `initializeForMode()`, but that is not the right
substitute for the spec sites — it needs an `IConnectionStrategy` +
`ConnectionConfig` pair, which would turn 8 lightweight setup lines into strategy
mocks and change what the tests exercise. The faithful, behaviour-preserving
retirement is:

1. Delete the method from **both** `content-resolver.service.ts` copies.
2. In `content-resolver.service.spec.ts`, replace the 8 call sites with a
   test-local helper that registers the identical set, preserving chain order:

   ```ts
   /** Test helper — replaces the retired registerAllStandardSources(). */
   function registerAllForTest(service: ContentResolverService): void {
     for (const id of ['indexeddb', 'projection', 'conductor', 'edgenode', 'dht', 'cdn']) {
       service.registerStandardSource(id);
     }
   }
   ```

3. Drop the now-redundant `it('should register all standard sources')` case at
   :240-247, which asserts nothing but `toBeTruthy()` on the deprecated method.

Behaviour-neutral by construction: no production code path changes.

**Sequencing note.** Do this *after* — or together with — the conductor concern
(`deprecation-conductor-content-tier-retirement-strategy-seam`). That work
deletes `STANDARD_SOURCES.conductor`, which is one of the six ids the helper
above registers; landing them in the wrong order leaves the helper referencing a
removed key and `registerStandardSource()` throws `Unknown standard source`.

## Current decision

Bounded, low-risk, ready to execute — **not landed on 2026-07-30 because the two
target files were under an active concurrent `LearningPath → PathView` rename
sweep** in the shared worktree on `feat/angular22-node24`. Editing them mid-sweep
would have forced this agent to commit another session's unfinished migration
hunks (they live in the same files, so no path-limited split is available), which
is a worse outcome than deferring a test-scaffolding cleanup with zero production
value at risk.

Next run: confirm `git status --short app/elohim-app/src/app/elohim/services/content-resolver.service.ts`
is clean (or that the PathView sweep has been committed), then execute the three
steps above. This is a mechanical change — it does not need a fresh scoping pass.

## Verification

Not yet fixed. Gate for the eventual change, baselined green on 2026-07-30:

```
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts \
  src/app/elohim/services/content-resolver.service.spec.ts
# 49/49 passing
```

The suite must still read 49/49 after the swap (the helper preserves every
registration), minus the one deleted no-op assertion → expect 48/48.
