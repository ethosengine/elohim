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
fingerprints: ["f484d562d2b3", "712613235841"]
relatedNodeIds: []
tags: [deprecation, typescript, angular, elohim-service, LocalSourceChainService, M-AGGR-2]
cites:
  - app/elohim-library/projects/elohim-service/src/angular/services/local-source-chain.service.ts
  - app/elohim-library/projects/elohim-service/src/angular/services/holochain-source-chain.service.ts
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

Active usages as of 2026-06-08:

- `app/elohim-app/src/app/elohim/services/human-consent.service.ts:20` — import
- `app/elohim-app/src/app/elohim/services/human-consent.service.ts:49` — `inject(LocalSourceChainService)` (read-path consumer)
- `app/elohim-app/src/app/elohim/services/human-consent.service.spec.ts:5,47,53,54` — test fixture stubs
- `app/elohim-app/src/app/elohim/services/index.ts:10-11` — re-export with migration comment
- `app/elohim-app/src/app/elohim/services/acquisition.service.spec.ts:11` — test-setup import (indirectly via service under test)

Note: the `acquisition.service.spec.ts` warning (fp 712613235841) and the
`services/index.ts` re-export warning (fp f484d562d2b3) are the same concern —
`LocalSourceChainService` imported via the barrel in test setup.

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

Consumer count is small (one service file + tests). The read-path consumer
(`human-consent.service.ts`) COULD be switched to `HolochainSourceChainService` today,
but the write-path entries (`createEntry` / `createLink`) used by `human-consent.service.ts`
have no substrate-coordinator equivalent until `M-REA-1` lands. Partial migration
(read-only swap) would leave the same service injecting both services — net negative
for clarity with near-zero ESLint benefit.

## Current decision

**Blocked.** Full migration gated on `M-REA-1` + `M-AGGR-1` substrate milestones
(write-path substrate coordinators for `createEntry`/`createLink` in `human-consent.service.ts`).
The `@deprecated` annotation is intentional architectural signalling, not an oversight.
The ESLint `@typescript-eslint/no-deprecated` warnings are expected suppressible noise
until the milestone clears. When `M-REA-1` + `M-AGGR-1` land, migrate
`human-consent.service.ts` to `HolochainSourceChainService`, delete the barrel re-export,
and close this entry by deleting both the ledger fingerprints and this file.

The sentinel will suppress further dispatch on these fingerprints (ledger status: blocked).

## Verification

N/A — not yet fixed. Will be verified when `M-REA-1` + `M-AGGR-1` land and
`human-consent.service.ts` test suite stays green after migration.
