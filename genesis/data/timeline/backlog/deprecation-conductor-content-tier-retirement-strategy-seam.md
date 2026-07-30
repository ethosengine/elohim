---
id: "backlog-deprecation-conductor-content-tier-retirement-strategy-seam"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Conductor content-tier retirement is half-landed — the connection strategies still advertise conductor for all content types"
slug: "deprecation-conductor-content-tier-retirement-strategy-seam"
written: "2026-07-30"
author: "deprecation-triage"
status: "backlog"
priority: "medium"
deprecation_status: blocked
severity: medium
fingerprints: ["d35a955ce973", "1bad960b8fcd", "00673a950e70"]
relatedNodeIds: []
tags: [deprecation, angular, elohim-app, lamad, elohim-service, content-resolver, connection-strategy, conductor, tauri, direct-mode]
cites:
  - app/elohim-app/src/app/elohim/services/content-resolver.service.ts
  - app/elohim-app/src/app/elohim/services/data-loader.service.ts
  - app/lamad/src/app/services/content-resolver.service.ts
  - app/lamad/src/app/services/data-loader.service.ts
  - app/elohim-library/projects/elohim-service/src/connection/doorway-connection-strategy.ts
  - app/elohim-library/projects/elohim-service/src/connection/direct-connection-strategy.ts
  - app/elohim-library/projects/elohim-service/src/connection/tauri-connection-strategy.ts
  - app/elohim-app/src/app/elohim/services/content-resolver.service.spec.ts
---

## What is deprecated

Two of the three captured fingerprints are the same retirement decision, written
twice in the app layer:

```
* @deprecated Conductor is no longer used for content resolution.
* Content is now served from doorway projection (SQLite).
* Conductor remains available for agent-centric data only (identity, attestations, points).
```

```
this.contentResolver.setSourceAvailable('conductor', false); // Conductor deprecated for content
```

The retirement landed in the **app** layer and stopped there. It never reached
the **library** layer that actually feeds the recommended replacement API.

## Usage inventory

**App layer — retirement applied (mirrored byte-for-byte in two workspaces):**

| Site | State |
|---|---|
| `app/elohim-app/src/app/elohim/services/content-resolver.service.ts:103-113` | `STANDARD_SOURCES.conductor` narrowed to `['identity','attestation','point-balance']`, carries the `@deprecated` block |
| `…/content-resolver.service.ts:640-647` | `isSourceReady()` hard-returns `false` for `conductor` — unconditional, ignores registered availability |
| `…/content-resolver.service.ts:664, 755, 817` | all three `case 'conductor':` fetch arms return `null` / empty `Map` |
| `app/elohim-app/…/data-loader.service.ts:282,287` | registers `conductor`, then immediately `setSourceAvailable('conductor', false)` |
| `app/lamad/src/app/services/content-resolver.service.ts` (same line numbers) | identical copy — the two resolver files differ **only** in two import paths |
| `app/lamad/src/app/services/data-loader.service.ts:293,298` | identical register-then-disable pair |

**Library layer — retirement NOT applied.** All three `IConnectionStrategy`
implementations still declare a `conductor` source carrying the *full* content
set `['path','content','graph','assessment','profile','identity']`:

| Strategy | Conductor priority |
|---|---|
| `doorway-connection-strategy.ts:271` | 50 |
| `direct-connection-strategy.ts:135` | 90 (above `elohim-storage`) |
| `tauri-connection-strategy.ts:350` | 90 (above `elohim-storage`) |

## Why this is a seam defect, not dead code

`initializeForMode(strategy, config)` — the API the *other* fingerprint in this
sweep tells callers to migrate to — registers sources straight from
`strategy.getContentSources(config)`. So the recommended path registers a
conductor source that **advertises every content type**, at priority 90 in
native modes. The only thing preventing a permanent dead hole at the top of the
native resolution chain is the hard-coded skip in `isSourceReady()`. That skip is
a compensating band-aid masking a source-of-truth contradiction, and it has a
second-order cost: because it is unconditional, it also kills conductor for
`identity` — falsifying the deprecation's own promise that "conductor remains
available for agent-centric data."

Latent, not live-breaking: grep confirms **nothing** in either workspace resolves
`identity`, `attestation`, or `point-balance` through `ContentResolverService`
(`resolve(…)` / `getResolutionChain(…)` are only ever called for content/path/
blob/app). Agent-centric reads go through `HolochainClientService` /
`StorageApiService` instead.

## Migration path

Delete the conductor source from the resolver seam rather than keep compensating:

1. Narrow or drop `conductor` in the three strategy `getContentSources()` lists.
2. Drop `STANDARD_SOURCES.conductor` and the three `case 'conductor':` arms in
   both `content-resolver.service.ts` copies.
3. Drop the register-then-disable pair in both `data-loader.service.ts` copies.
4. Remove the `isSourceReady()` special-case — it becomes unreachable, and
   leaving it in place is what let the contradiction hide.
5. Update `content-resolver.service.spec.ts` (mock at :110, registration at :220,
   skip-behaviour comments at :278/:316/:362/:527).

Roughly nine files, no dependency-version movement — inside a background agent's
bounded envelope on the code mechanics alone.

## Current decision

**BLOCKED on an architecture answer that this agent cannot supply: what serves
`path`/`content`/`graph`/`assessment`/`profile` in direct/Tauri (native) mode
once conductor is narrowed?**

Both native strategies declare `elohim-storage` with `contentTypes: ['blob']`
only. Remove conductor's content types and the native resolution chain has
`indexeddb` (local cache) and then *nothing* authoritative for content — whereas
doorway mode correctly falls to `projection`. Either (a) `elohim-storage` should
advertise the full content set in native modes and this is a one-line widening,
or (b) native content deliberately bypasses `ContentResolverService` via
`DataLoaderService` → `ContentService`, and the resolver's native chain is
vestigial. The evidence is consistent with (b) — the unconditional skip means
native content resolution through this service already returns nothing today —
but confirming that is an `angular-architect` + `rust-architect` call about the
Tauri sidecar read path, not a deprecation-sweep call.

Secondary, non-blocking constraint recorded on 2026-07-30: both
`content-resolver.service.ts` / `data-loader.service.ts` copies were under an
active concurrent `LearningPath → PathView` rename sweep in the shared worktree
on `feat/angular22-node24`. Any run that picks this up should re-check
`git status` on the five app-layer files first and commit path-limited.

## Verification

Not yet fixed — no verification to record. Baseline captured for the next run:
`pnpm exec vitest run --config vite.config.ts src/app/elohim/services/content-resolver.service.spec.ts`
was **49/49 passing** in `app/elohim-app` on 2026-07-30, so the spec is a usable
green gate for the eventual change.
