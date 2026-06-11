---
id: "backlog-reach-vocabulary-frontend-strand"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Reach enum drift has a 4th (and 5th) vocabulary: the TypeScript geographic family — unrecorded in the canonical reconciliation item"
slug: "reach-vocabulary-frontend-strand"
written: "2026-06-11"
author: "claude (elohim-pillar island recompose)"
status: "backlog"
priority: "medium"
tags: [reach, vocabulary-drift, frontend, sdk, reconciliation, lamad, elohim-library, storage-client-ts]
derived_from:
  - app/elohim-app/src/app/elohim/ARCHITECTURE.md   # retired to git 2026-06-11 (elohim-pillar island recompose) — carried the geographic 8 verbatim
  - elohim/elohim-storage/REACH.md                  # retired to git 2026-06-11 (storage island recompose) — design-doc ORIGIN of the geographic 8 (§Core Mapping carries the ladder verbatim)
  - doorway/doorway-service/REACH.md                # retired to git 2026-06-11 (doorway island recompose) — sibling origin-strand; same geographic 8 as its enforcement ladder + access matrix
cites:
  - resilience-protocol-spec | the canonical reconciliation home — gap-matrix row :628 + roadmap item 13 :704 name only three of the (now five+) reach vocabularies | sha256:2c832b517c7204cc | path: genesis/docs/content/elohim-protocol/resilience/README.md
  - genesis/data/timeline/backlog/http-reach-enforcement-gap.md
  - app/elohim-app/src/app/elohim/models/protocol-core.model.ts
  - elohim/sdk/storage-client-ts/src/protocol-core.model.ts
  - app/lamad/src/app/models/trust-badge.model.ts
  - app/elohim-library/projects/elohim-service/src/cache/types.ts
  - app/elohim-library/projects/elohim-service/src/models/holochain.model.ts
  - elohim/sdk/schemas/v1/enums/reach.schema.json
---

# Reach drift: the TypeScript geographic vocabulary is an unrecorded 4th strand

The canonical reconciliation item — resilience README gap-matrix row (`genesis/docs/content/elohim-protocol/resilience/README.md:628`) and roadmap item 13 (`:704`) — names THREE vocabularies: Rust services enum (`elohim/elohim-storage/src/services/epr_kind.rs:88-97` — personal/intimate/household/neighborhood/collective/community/district/public), schema enum (`elohim/sdk/schemas/v1/enums/reach.schema.json` — private/self/intimate/trusted/familiar/community/public/commons; matched by `elohim/epr/src/reach.rs:18-37`), and resilience-epic Part V (household/neighborhood/community/organization/commons).

The TypeScript side carries a **4th vocabulary, unrecorded there**: the 8-value GEOGRAPHIC family `private/invited/local/neighborhood/municipal/bioregional/regional/commons`, defined at FOUR sites:

1. `app/elohim-app/src/app/elohim/models/protocol-core.model.ts:50-72` (+ `reachEncompasses()` ordinal comparison)
2. `elohim/sdk/storage-client-ts/src/protocol-core.model.ts:50-124` — the SDK twin exporting `ReachLevel`, `REACH_LEVEL_VALUES`, `reachEncompasses()`; this is what the lamad bundle imports (`app/lamad/src/app/models/content-node.model.ts:31`, `app/lamad/src/app/quiz-engine/services/discovery-attestation.service.ts:20`, etc.)
3. `app/lamad/src/app/models/trust-badge.model.ts:20-28` (inlined copy)
4. `app/elohim-library/projects/elohim-service/src/cache/types.ts:19-40` (numeric const 0-7, same vocabulary — feeds the reach-aware cache)

**Design-doc origin (noted 2026-06-11, storage island recompose):** the geographic 8 is not a
frontend invention — it originates in `elohim/elohim-storage/REACH.md` (2026-04-15, the storage
data-plane reach design; retired to git 2026-06-11). Its §"The Core Mapping: Reach → Trust →
Action" table carries the ladder verbatim, driving designed-but-never-built encryption/eviction/
replication tiers (residue: `genesis/data/timeline/backlog/storage-island-harvest-residue.md`).
The four TS sites are descendants of that design, not drift that arose frontend-side.

**Sibling origin-strand (noted 2026-06-11, doorway island recompose):** `doorway/doorway-service/REACH.md`
(retired to git 2026-06-11; its `../REACH.md` "system-wide" pointer was already dead) carries the SAME
geographic 8 as its HTTP enforcement ladder + access matrix. Doorway's live code knows the full 8 —
`REACH_LEVELS` + `can_serve_at_reach` in `doorway/doorway-service/src/cache/access_control.rs`
(private → beneficiary match; invited → "simplified: authenticated only"; local…regional → any
authenticated; commons → everyone; unknown → deny) — but enforces LESS than REACH.md documented:
no invite-list, no relationship check, no `REACH_DENIED` error code anywhere in src/, and the only
`X-Reach` response header is hardcoded `"commons"` (`doorway/doorway-service/src/routes/apps.rs:355`).
The machinery is zero-consumer beyond its own module: `can_serve_at_reach` is called only from
`src/cache/reach_aware_serving.rs`, whose functions have no callers outside `src/cache/` — no doorway
HTTP route gates by reach this way. The live enforcement gap is already tracked in
`genesis/data/timeline/backlog/http-reach-enforcement-gap.md` (do not re-fork it here). The "7th mixed
variant" table noted in the blast-radius paragraph below (`doorway/CLAUDE.md:139-144`) was corrected to
the geographic 8, with accurate simplified-enforcement language, in the 2026-06-11 doorway island recompose.

And a **5th, mutually inconsistent** 6-value family `private/invited/local/community/federated/commons`:

5. `app/elohim-library/projects/elohim-service/src/models/holochain.model.ts:319-326` — `VALID_REACH_LEVELS`, comment claims "matching Rust validation" (false: matches neither Rust enum)
6. ~~`app/elohim-app/src/app/elohim/CLAUDE.md` "Key Types" block (same 6 values)~~ — corrected to the geographic 8 (with a drift annotation) in the elohim-pillar island recompose 2026-06-11; the `VALID_REACH_LEVELS` code site remains.

Cross-bundle blast radius: 72 files reference `ReachLevel` across app/elohim-app, app/lamad, app/elohim-library (incl. dist/spec); 96 non-spec literal usages of geographic values (`bioregional`/`municipal`/`neighborhood`) across the three trees, e.g. `app/lamad/src/app/models/content-attestation.model.ts:76` maps attestation types to geographic reaches, `app/lamad/src/app/services/content.service.ts:97,358` defaults to `bioregional`. ~~Doorway's documented table is a 7th mixed variant (`doorway/CLAUDE.md:139-144` — commons/regional-private/local/private)~~ — corrected to the geographic 8 (with accurate simplified-enforcement language) in the doorway island recompose 2026-06-11; the gospel no longer carries the mixed 4-row table.

**Why this matters for roadmap item 13**: a reconciliation scoped only to Rust/schema/epic will under-scope. The TS geographic ordinals feed `reachEncompasses()` comparisons and reach-aware cache eviction; renaming the vocabulary changes ordinal semantics across ~70 consuming files in three separately-built bundles, two of which (lamad via `@elohim/storage-client`, elohim-library locally) cannot be fixed by editing elohim-app alone.

**Action**: when roadmap item 13 is picked up, extend the gap-matrix row at `resilience/README.md:628` to name the TS strands, and treat `elohim/sdk/storage-client-ts/src/protocol-core.model.ts` (hand-written, NOT ts-rs-generated despite living in the SDK) as the single TS edit point — the other three sites should re-export rather than redefine.
