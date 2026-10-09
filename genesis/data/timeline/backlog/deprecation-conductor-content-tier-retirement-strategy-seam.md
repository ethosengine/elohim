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
deprecation_status: open
severity: medium
fingerprints: ["d35a955ce973", "1bad960b8fcd", "00673a950e70", "365d2205aaf8", "ba65f8c7be09"]
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
  - app/elohim-app/src/app/elohim/services/holochain-client.service.ts
  - genesis/data/timeline/backlog/deprecation-content-resolver-register-all-standard-sources-retire.md
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
| `app/elohim-app/…/data-loader.service.ts:293` (the disable call; it was at :287) | registers `conductor`, then immediately `setSourceAvailable('conductor', false)` |
| `app/lamad/src/app/services/content-resolver.service.ts` (same line numbers) | identical copy — the two resolver files differ **only** in two import paths |
| `app/lamad/src/app/services/data-loader.service.ts:294` + `:299` (register, then disable; it was at :298) | identical register-then-disable pair |

**Library layer — retirement NOT applied.** All three `IConnectionStrategy`
implementations still declare a `conductor` source carrying the *full* content
set `['path','content','graph','assessment','profile','identity']`:

| Strategy | Conductor priority |
|---|---|
| `doorway-connection-strategy.ts:343` | 50 |
| `direct-connection-strategy.ts:149` | 90 (above `elohim-storage`) |
| `tauri-connection-strategy.ts:362` | 90 (above `elohim-storage`) |

## Why the seam looked live (2026-07-30 reading; superseded by Current decision)

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

**Re-decided 2026-10-08 (fingerprint `ba65f8c7be09`): the conductor content
source is dead code to remove, not a declared non-source. The architecture
question that blocked this item is answered by the tree itself.** The fix is
queued for the session that owns `app/lamad/src/app/services/data-loader.service.ts`.
It was not landed here because that file was being edited concurrently.

The blocking question was: what serves content in direct/Tauri (native) mode
once conductor is narrowed? A caller census of the current tree answers it with
option (b). Native content never goes through the resolver's strategy-sourced
chain:

- `ContentResolverService.initializeForMode()` is the only path that copies a
  strategy's `getContentSources()` into the resolver. It has **zero production
  callers** in either workspace. Its only mention outside its own definition is
  the JSDoc example at `content-resolver.service.ts:334`.
- `HolochainClientService.getContentSources()` (`holochain-client.service.ts:134`),
  the other place that reads the strategy lists, also has **zero production
  callers**. Only its spec reaches it.
- Production registers exactly `indexeddb`, `projection` (when enabled) and
  `conductor` in `data-loader.service.ts`, then disables `conductor` on the
  next statement. The resolver is used in production only for the
  fire-and-forget `resolveContent(id)` prefetch (`app/lamad` :571,
  `app/elohim-app` :565).
- Authoritative content reads go through `DataLoaderService.getContent()`. It
  tries projection first, then falls back to `ContentService`, in every mode.
  The resolver is not on that path.

So the strategy lists' `conductor` entry at priority 90 never reaches a running
resolver. The `isSourceReady()` skip and the register-then-disable pair guard a
path nobody takes. Removing conductor changes no production behaviour. A native
"content hole" cannot open, because native content does not resolve through
this seam today.

**Trajectory for the owning session** (one slice, about 10 files, no
dependency movement). Land it together with
`deprecation-content-resolver-register-all-standard-sources-retire`. Its test
helper registers `conductor`, so landing the two in the wrong order makes
`registerStandardSource()` throw `Unknown standard source`.

1. `app/lamad/src/app/services/data-loader.service.ts:294,299` and
   `app/elohim-app/src/app/elohim/services/data-loader.service.ts:288,293`:
   delete both the `registerStandardSource('conductor')` line and the
   `setSourceAvailable('conductor', false)` line.
2. Both `content-resolver.service.ts` copies: delete `STANDARD_SOURCES.conductor`
   (:108), the `isSourceReady()` special case (:642), the three
   `case 'conductor':` arms, and `conductor` from the chain-order doc comments
   (:320, :324). In the same pass, delete `registerAllStandardSources()` (the
   sister entry).
3. The three strategy `getContentSources()` lists: drop the `conductor` entry.
   Its last surviving claim is `identity`, and nothing resolves `identity`
   through the resolver (the 2026-07-30 grep still holds).
4. `content-resolver.service.spec.ts`, plus the strategy and holochain-client
   specs if they assert on the conductor entry: update the mocks and expectations.
5. Open follow-on question, outside this slice: `initializeForMode()` and
   `getContentSources()` are themselves caller-less. Once conductor is gone,
   decide whether mode-aware registration has a consumer or should be retired.
   Until then the sister entry's "use `initializeForMode()`" deprecation points
   at an API that nothing uses.

Fingerprint history: `ba65f8c7be09` (2026-10-08) is the same
`setSourceAvailable('conductor', false)` line again, captured as `299:` from a
`sed -n` read of `app/lamad/…/data-loader.service.ts`. It is the third
line-number-prefixed recapture of the same line (after `00673a950e70` and
`365d2205aaf8`), which is the sentinel's Class 3 instability. Deleting the line
in step 1 ends the recaptures.

## Verification

Not yet fixed — no verification to record. Baseline captured for the next run:
`pnpm exec vitest run --config vite.config.ts src/app/elohim/services/content-resolver.service.spec.ts`
was **49/49 passing** in `app/elohim-app` on 2026-07-30 and again on 2026-10-08
(3.22 s, exit 0), so the spec is a usable green gate for the eventual change.
When the slice lands, also run the lamad workspace gate (`just gate` covering
`app/lamad`) and the `elohim-service` connection-strategy spec.
