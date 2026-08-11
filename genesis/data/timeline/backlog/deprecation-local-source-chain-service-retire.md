---
id: "backlog-deprecation-local-source-chain-service-retire"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Retire LocalSourceChainService (M-AGGR-2 localStorage simulation)"
slug: "deprecation-local-source-chain-service-retire"
written: "2026-06-08"
author: "deprecation-triage"
status: "backlog"
priority: "medium"
deprecation_status: blocked
severity: low
fingerprints: ["f484d562d2b3", "712613235841", "535658148aaf", "0abbd0ab4e34", "0a652fe0dc60"]
relatedNodeIds: []
tags: [deprecation, typescript, angular, elohim-service, lamad, LocalSourceChainService, M-AGGR-2]
cites:
  - app/elohim-library/projects/elohim-service/src/angular/services/local-source-chain.service.ts
  - app/elohim-library/projects/elohim-service/src/angular/services/holochain-source-chain.service.ts
  - app/elohim-app/src/app/elohim/services/human-consent.service.ts
  - app/lamad/src/app/services/mastery-stats.service.ts
  - app/lamad/src/app/services/path-negotiation.service.ts
  - app/lamad/src/app/services/content-mastery.service.ts
  - genesis/docs/superpowers/plans/2026-05-28-thin-client-backend-migration.md
---

## What is deprecated

```
warning  `LocalSourceChainService` is deprecated. M-AGGR-2: LocalSourceChainService is a
localStorage simulation that was always meant to retire when Holochain was ready.
Use `HolochainSourceChainService` for source-chain reads.
```

The `@deprecated` annotation is intentional — placed by the substrate engineers in
`app/elohim-library/projects/elohim-service/src/angular/services/local-source-chain.service.ts`
to mark the migration trajectory, not a surprise regression.

## Usage inventory

Refreshed 2026-06-27 (the 2026-06-08 pass understated the blast radius — the
lamad pillar has since added three production consumers, and two new barrel
surfaces were captured: the elohim-service library barrel and its ng-packagr
public-api). **Production consumers (all use both read AND write paths):**

- `app/elohim-app/src/app/elohim/services/human-consent.service.ts:20,49` — import + `inject`; reads via `getEntriesByType`/`getAgentId`, writes via `createEntry('human-consent', …)` (line 513)
- `app/lamad/src/app/services/mastery-stats.service.ts:26,76` — import + `inject`; writes via `createEntry(ENTRY_TYPE_STREAK, …)` (line 236) and `createEntry(ENTRY_TYPE_LEVEL_UP, …)` (line 354); reads via `getEntriesByType`
- `app/lamad/src/app/services/path-negotiation.service.ts:17,55` — import + `inject`; writes via `createEntry('path-negotiation', …)` (line 638); reads via `getEntriesByType`/`getAgentId`
- `app/lamad/src/app/services/content-mastery.service.ts:14,78` — import + `inject`; writes via `createEntry<MasteryRecordContent>(…)` (line 326); reads via `getEntriesByType` + `initializeForAgent`

**Barrel / public-API re-export surfaces:**

- `app/elohim-library/projects/elohim-service/src/index.ts:161` — package barrel re-export with the `@deprecated M-AGGR-2` JSDoc (fp **535658148aaf**, captured 2026-06-27 from a `cat src/index.ts`)
- `app/elohim-library/projects/elohim-service/src/public-api.ts:34` — ng-packagr public API re-export
- `app/elohim-app/src/app/elohim/services/index.ts:10-11` — re-export with migration comment (fp f484d562d2b3)

**Deprecated source + its own test:**

- `app/elohim-library/projects/elohim-service/src/angular/services/local-source-chain.service.ts:18,47` — the `@deprecated` annotation + class definition (fp **0abbd0ab4e34**, captured 2026-06-27 from a `grep`/scope pass over the source)
- `app/elohim-library/projects/elohim-service/src/angular/services/local-source-chain.service.spec.ts` — the service's own unit test (retires with the class)

**Test fixtures (retire with their services-under-test):**

- `app/elohim-app/src/app/elohim/services/human-consent.service.spec.ts:5,47,53,54`
- `app/elohim-app/src/app/elohim/services/acquisition.service.spec.ts:11`
- `app/lamad/src/app/services/{content-mastery,mastery-stats,path-negotiation}.service.spec.ts` — DI provider stubs

Note: all four 2026-06-08/2026-06-27 fingerprints are the **same concern** —
`LocalSourceChainService` surfaced via ESLint `no-deprecated` (f484d562d2b3,
712613235841), the library barrel JSDoc (535658148aaf), and the source
annotation itself (0abbd0ab4e34). The last two are sentinel self-captures of
agent scope passes (`cat`/`grep`) over the intentional `@deprecated` marker, not
new build-tool warnings — folded here so the sentinel cites this blocked
decision deterministically rather than re-dispatching.

## Migration path

The class docstring documents the three-phase retirement:
1. **Phase G (M-AGGR-2)** — `HolochainSourceChainService` created (read paths ready).
   `HolochainSourceChainService` already exists at
   `app/elohim-library/projects/elohim-service/src/angular/services/holochain-source-chain.service.ts`.
2. **Wave B/C (M-REA-1, M-AGGR-1)** — write-path consumers (`createEntry`, `createLink`)
   migrate to substrate coordinators. Blocked on the substrate primitives landing in
   `mishpat` + the REA commitment write-path (`M-REA-1`).
3. **Final deletion** — once all consumers (currently `human-consent.service.ts` read +
   write paths) are migrated.

**`HolochainSourceChainService` is read-only** (verified 2026-06-27 — its only
public methods are `getEntries`, `getLinks`, `filterByType`; no `createEntry`
equivalent). **Every** production consumer (now FOUR services, not one) calls
`createEntry(...)` on the write path:

- `human-consent.service.ts` → `createEntry('human-consent', …)`
- `mastery-stats.service.ts` → `createEntry(ENTRY_TYPE_STREAK/LEVEL_UP, …)`
- `path-negotiation.service.ts` → `createEntry('path-negotiation', …)`
- `content-mastery.service.ts` → `createEntry<MasteryRecordContent>(…)`

A read-only swap on any of them would leave that service injecting BOTH classes —
the same net-negative the 2026-06-08 decision rejected, now multiplied across the
lamad pillar. The migration must be coordinated and write-path-complete, which is
exactly what `M-REA-1` sequences.

## Current decision

**Blocked** (re-confirmed 2026-06-27). Full migration is gated on the `M-REA-1`
substrate keystone — server-composed `EconomicEvent`/intent write-path coordinators
that replace `createEntry`/`createLink` — plus `M-AGGR-1` (`SessionHumanView`
projection). Both are still PLANNED, not landed
(`genesis/docs/superpowers/plans/2026-05-28-thin-client-backend-migration.md`;
M-REA-1 is the named KEYSTONE most other tickets depend on). `HolochainSourceChainService`
provides reads today but no write path, so no consumer can fully cut over.

The `@deprecated` annotation is intentional architectural signalling, not an
oversight; the ESLint `@typescript-eslint/no-deprecated` warnings (and the
sentinel's `cat`/`grep` self-captures of the marker) are expected suppressible
noise until the milestone clears. When `M-REA-1` + `M-AGGR-1` land: migrate all
four consumers to the substrate write-path + `HolochainSourceChainService` reads,
delete both barrel re-exports + the ng-packagr public-api line, delete
`local-source-chain.service.ts` (+ its spec), then close this entry by deleting
the four ledger fingerprints and this file.

The sentinel will suppress further dispatch on all four fingerprints (ledger
status: blocked).

## Verification

N/A — not yet fixed (blocked on substrate milestones). Will be verified when
`M-REA-1` + `M-AGGR-1` land and the four consumers' Vitest suites stay green after
migration (`pnpm --filter lamad test` + `app/elohim-app` `pnpm test`).
