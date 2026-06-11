---
id: "backlog-seeder-onto-doorway-session-client"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Migrate genesis/seeder auth onto @elohim/identity DoorwaySessionClient"
slug: "seeder-onto-doorway-session-client"
written: "2026-06-11"
author: "claude (jenkins-seed-bearer-gate plan Task 2)"
status: "backlog"
priority: "low"
relatedNodeIds: []
tags: [genesis-seeder, auth, sdk-consolidation, doorway-session-client, jenkins-seed-bearer-gate]
cites:
  - genesis/seeder/src/doorway-client.ts
  - app/elohim-library/projects/elohim-identity/src/lib/doorway-session-client.ts
  - genesis/docs/superpowers/plans/2026-06-11-jenkins-seed-bearer-gate-plan.md
  - genesis/data/timeline/backlog/angular-auth-onto-doorway-session-client.md
---

# Migrate the seeder onto DoorwaySessionClient

`DoorwaySessionClient` (`@elohim/identity/core`) is the consolidated SDK home for
the doorway auth surface — the a2o framework already consumes it (the framework-free
`/core` import, proven Node-consumable). The jenkins-seed-bearer-gate plan Task 2
deliberately took the **minimal-login** path instead: it added a hand-rolled
`login()` (POST `/auth/login`, stored JWT) to the seeder's own
`genesis/seeder/src/doorway-client.ts` rather than wiring in the SDK client.

## Why minimal was chosen (Task 2 decision, journaled)

1. **No workspace dep today.** The seeder's `package.json` deps are
   `@elohim/storage-client`, `@holochain/client`, `multiformats`, `ws`, `yaml` —
   `@elohim/identity` is net-new wiring.
2. **The seeder already hand-rolls auth in every sibling** (`seed-test-admin.ts`,
   `seed-commitments.ts`, `seed-projections.ts`, `seed-operator-bindings.ts` all
   read `DOORWAY_API_KEY` and attach a bearer). The seeder's own `DoorwayClient.fetch()`
   already has an `Authorization: Bearer` slot — filling it with a `login()` JWT
   is the smallest honest change and matches the existing pattern.
3. **Angular peer-dependency surface.** `@elohim/identity` declares `@angular/*`
   + `rxjs` peerDependencies. The `/core` subpath is framework-free by design and
   a2o proves it under `tsx`, but the seeder consumes packages via bare `tsx` +
   NodeNext source-pointed `exports` — first-time `@elohim/identity` consumption
   in the seeder is more wiring than this slice warranted.

## The migration (this item)

Replace the seeder's hand-rolled `login()` + JWT field in `doorway-client.ts`
with `DoorwaySessionClient` from `@elohim/identity/core`:

- Add `@elohim/identity: workspace:*` to `genesis/seeder/package.json`.
- Hold a `DoorwaySessionClient` instance; `login()` / `restoreSession()` /
  (future) `refresh()` come for free; the typed `DoorwaySessionError` replaces
  the string-built error messages.
- Keep the `bearerToken` / `setBearerToken` seam that lets `seed.ts` share one
  seed-start JWT across the blob-PUT client and the raw `/admin/cache/warm` fetch.
- Token refresh (out of scope in Task 2 — seed runs are short) becomes trivial
  once the SDK client is in, via `DoorwaySessionClient.refresh()`.

Sibling of `angular-auth-onto-doorway-session-client.md` (same SDK consolidation,
different consumer). Low priority: the minimal `login()` is correct and tested;
this is consolidation hygiene, not a correctness gap.
