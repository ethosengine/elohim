---
id: "backlog-deprecation-lamad-legacy-path-model-aliases-retire"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Retire the legacy lamad path-model aliases (LearningPath → PathView, PathChapter → PathSection)"
slug: "deprecation-lamad-legacy-path-model-aliases-retire"
written: "2026-07-30"
author: "deprecation-triage"
status: "wip"
priority: "medium"
deprecation_status: "in-progress"
severity: "low"
fingerprints: [257791f79370, 816530d7b1aa, 041d752be688, 5b4f9469dd9a, 5e199d996bcc, 46083d688b4e, 10619cea8076, 7ac0686a3992, b3dd82fe2343]
relatedNodeIds: []
tags: [deprecation, lamad, path-view, typescript, eslint, no-deprecated, sonarjs]
cites:
  - app/lamad/src/app/models/learning-path.model.ts
  - app/elohim-app/src/app/elohim/services/projection-api.service.ts
  - app/elohim-app/src/app/elohim/services/data-loader.service.ts
  - genesis/data/timeline/backlog/deprecation-learning-path-zome-surface-retire.md
---

# Retire the legacy lamad path-model aliases

## What is deprecated

Two TypeScript-side legacy declarations in the "Legacy Types — backward-compatible
aliases and synthesized types" block of `app/lamad/src/app/models/learning-path.model.ts`:

> ```ts
> /**
>  * @deprecated Use PathView instead. This alias exists for backward compatibility.
>  */
> export type LearningPath = PathView;
> ```

> ```ts
> /**
>  * PathChapter — synthesized from top-level sections for backward compat.
>  *
>  * @deprecated Use PathSection with level='course' instead.
>  */
> export interface PathChapter { … }
> ```

Both fire TWO ESLint rules on every reference: `@typescript-eslint/no-deprecated`
(warning) **and** `sonarjs/deprecation` (error). The aggregate ledger capture
(`257791f79370` = `34 @typescript-eslint/no-deprecated`) was elohim-app's whole
`no-deprecated` count, and **32 of those 34 were `LearningPath`** — the other 2
belong to the separately-canonicalized `LocalSourceChainService` concern
(`deprecation-local-source-chain-service-retire.md`).

This is a **distinct concern from** `deprecation-learning-path-zome-surface-retire.md`.
That entry covers the Rust `content_store` zome's `LearningPath`/`PathStep`/`PathChapter`
DHT entry types and their SDK write surface. This entry covers only the Angular
**TypeScript type aliases** in the lamad model layer. The two share a name and an
architectural direction but not an upgrade unit.

**Do not confuse the alias with the wire name.** The string `'LearningPath'` is
also the live document-type name in doorway/projection URLs
(`buildApiUrl('/LearningPath')`, `query<T>('LearningPath', …)`,
`/cache/LearningPath/<id>`). Those are wire identifiers owned by the zome entry
type and must NOT be renamed by this concern — only bare identifier/type
positions migrate.

## Usage inventory

`LearningPath` alias, source files (excludes `coverage/`, `dist/`, ts-rs `bindings/`):

- **elohim-app — 12 files, MIGRATED this run** (`app/elohim-app/src/app/elohim/services/`):
  `content-resolver.service.ts` · `content.service.ts` + `.spec.ts` ·
  `data-loader.service.ts` + `.spec.ts` · `human-consent.service.ts` + `.spec.ts` ·
  `indexeddb-cache.service.ts` + `.spec.ts` · `profile.service.ts` ·
  `projection-api.service.ts` + `.spec.ts`
- **elohim-app — deliberately untouched** (wire-name / prose only, not the alias):
  `doorway-cache.service.ts` L73/L133/L140 + `doorway-cache.service.spec.ts` (wire name),
  `qahal/models/governance-feedback.model.ts` L44 (comment).
- **app/lamad — 27 files, REMAINING**: `models/learning-path.model.ts` (declaration),
  `models/index.ts`, `models/path-extension.model.ts`,
  `services/{projection-api,data-loader,path,path-graph,path-extension,path-negotiation,content,content-backend,content-resolver,indexeddb-cache,hierarchical-graph}.service.ts`
  (+ 8 matching `.spec.ts`), `quiz-engine/services/question-pool.service.ts` (+ spec),
  `content-io/services/content-editor.service.ts`,
  `components/path-{navigator,overview}/*.component.ts` (+ specs).
- **Not this concern** (these are the ts-rs *wire* `LearningPath`, not the alias):
  `elohim/sdk/src/types.ts`, `genesis/seeder/src/{validators,migrate-relationships}.ts`,
  `app/elohim-library/projects/elohim-service/src/services/doorway-client.service.ts`.

`PathChapter`, source files — **all REMAINING, all inside app/lamad** (zero elohim-app
references, so zero lint findings today): `models/learning-path.model.ts` (declaration),
`services/path.service.ts` (8), `components/path-navigator/path-navigator.component.ts` (5),
`components/path-overview/path-overview.component.ts` (4),
`services/hierarchical-graph.service.ts` (+ spec), `services/path-graph.service.spec.ts`.

## Migration path

`LearningPath` is a **pure type alias** (`export type LearningPath = PathView`), so
substituting `PathView` at identifier/type positions is type-identical and carries
zero runtime change. The guard is the wire-name collision: rename only occurrences
NOT adjacent to `/` (i.e. skip `/LearningPath`, `LearningPath/`) and not inside
quotes. That guard is what the landed elohim-app pass used, and it correctly left
all 20 wire-name references intact.

`PathChapter` is **not** a pure alias — it is a synthesized interface, and the
replacement (`PathSection` with `level='course'`) has a different shape. Its
migration requires reading each consumer's chapter-shaping logic (notably
`path.service.ts` and the two path components), so it is a real refactor, not a
substitution.

Remaining steps to close:

1. Sweep `app/lamad` for the `LearningPath` alias with the same wire-name-guarded
   substitution (27 files, mechanical).
2. Refactor the `PathChapter` consumers onto `PathSection` + `level='course'`
   (7 files, genuine shape change — needs the lamad path-model owner).
3. Delete both declarations from
   `app/lamad/src/app/models/learning-path.model.ts` and drop them from
   `models/index.ts`.
4. Verify: `pnpm test` in `app/lamad`, `pnpm run lint` + `pnpm test` in
   `app/elohim-app`, and confirm the elohim-app `no-deprecated` /
   `sonarjs/deprecation` counts for both symbols read 0.

## Current decision

**IN PROGRESS — the elohim-app half is LANDED and verified; the lamad half is
deferred to an operator-initiated sprint.**

Landed this run: all 32 `LearningPath` alias references in elohim-app's elohim-pillar
services migrated to `PathView` (12 files), removing 32 `@typescript-eslint/no-deprecated`
warnings **and** the matching `sonarjs/deprecation` **errors** from the elohim-app
lint gate. Wire-name references were preserved by the adjacency guard.

Deferred and why: the remainder is 27 lamad files for `LearningPath` plus a
genuine `PathChapter`→`PathSection` shape refactor. As one unit that exceeds the
>20-file background-agent ceiling, and `app/lamad` is under heavy in-flight
modification on `feat/angular22-node24` (the Angular 22 / Node 24 upgrade), so a
27-file sweep there would collide with live work. It is also not lint-visible:
`app/lamad`'s `pnpm lint` runs only `lint-route-literals` + `lint-ssr-entry`, no
ESLint — so the lamad alias usages emit no sentinel signal and carry no gate debt
today. Step 2 additionally needs the lamad path-model owner's judgment, not a
mechanical pass.

The sentinel cites THIS decision on every re-encounter of the nine fingerprints.
The stasis sweep re-checks once the Angular 22 branch settles; the natural moment
to run steps 1–4 is alongside the next lamad path-model change.

## Verification

Verified 2026-07-30 for the landed elohim-app half only:

- `pnpm exec eslint` on the 7 touched service files: `@typescript-eslint/no-deprecated`
  34 → 1, `sonarjs/deprecation` → 2. All 3 residuals are `LocalSourceChainService`
  in `human-consent.service.ts` (L20, L49) — a different, already-blocked concern.
  **Zero `LearningPath` deprecation findings remain.**
- `pnpm exec vitest run` on the 8 affected spec files: **334 tests passed, 0 failed**
  (6 files / 245 tests, then 2 files / 89 tests), exit 0.
- `pnpm exec prettier --check` on all 12 touched files: clean (one reformat applied
  to `content-resolver.service.ts` L743 — the shorter type name let a wrapped
  signature rejoin one line).

Closure of this concern happens when steps 1–4 land and both declarations are
deleted; at that point the nine ledger lines are removed and this entry is
deleted, the commit being the record.
