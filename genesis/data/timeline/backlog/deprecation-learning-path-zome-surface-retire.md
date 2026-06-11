---
id: "backlog-deprecation-learning-path-zome-surface-retire"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Retire deprecated LearningPath/PathStep/PathChapter zome surface (paths are Content now)"
slug: "deprecation-learning-path-zome-surface-retire"
written: "2026-06-11"
author: "deprecation-triage"
status: "backlog"
priority: "low"
deprecation_status: "blocked"
severity: "low"
fingerprints: [4889dfbac5bb, 9bcf23319dde, 7ed1280ab87e, 31e6d68d12f3, ebc68ea93525, 360b6083b7cd]
relatedNodeIds: []
tags: [deprecation, learning-path, content-store, zome, ts-rs, sdk, seeder, dna-migration]
cites:
  - elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
  - elohim/sdk/src/client/zome-client.ts
  - elohim/sdk/src/services/path.service.ts
  - genesis/seeder/src/seed-production.ts
  - genesis/seeder/src/add-elohim-steps.ts
  - genesis/seeder/src/seed.ts
  - genesis/docs/superpowers/specs/2026-06-08-native-content-graph-seam-design.md
---

# Retire deprecated LearningPath / PathStep / PathChapter zome surface

## What is deprecated

A deliberate, structured architectural deprecation in the `content_store` zome.
Paths are now `Content` entries with `content_type = "path"` and
`content_format = "epr-composite"`; the legacy `LearningPath` / `PathStep` /
`PathChapter` entry-type surface is retained only for reading existing DHT
entries during migration. The deprecated write functions are kept in the HDK
dispatch table but **return errors on invocation**.

The block header (content_store/src/lib.rs ~L3701):

> ```
> // Learning Path Operations (DEPRECATED)
> //
> // Paths are now Content entries with content_type "path" and content_format
> // "epr-composite". Write operations return deprecation errors. Read operations
> // are retained for accessing existing DHT entries during migration.
> ```

Representative captured warnings (the ledger fingerprints):

- `// Learning Path Operations (DEPRECATED)` (9bcf23319dde)
- `/// DEPRECATED: Use create_content with content_type "path" instead.` (7ed1280ab87e, 4889dfbac5bb)
- `"LearningPath is deprecated. Use Content with content_type 'path' and content_format 'epr-composite' instead."` (31e6d68d12f3)
- `"PathStep is deprecated. Use Content with content_type 'path' and Relationship entries instead."` (ebc68ea93525)
- `"batch_add_path_steps is deprecated. ..."` (360b6083b7cd)

These are NOT tooling/config/package deprecation warnings — they are
self-authored `DEPRECATED` doc-comments and runtime error strings that the
sentinel grep happened to scan from zome source. The code compiles clean
(22 `DEPRECATED` markers total in the file); the markers are intentional
documentation of an in-progress migration, not a broken state.

## Usage inventory

Deprecated zome functions (all in `content_store/src/lib.rs`):

- Write (return deprecation errors): `create_path` (L3711), `add_path_step`
  (L3720), `batch_add_path_steps` (L3738), `create_path_chapter` (L4015),
  `update_path` (L4236), `update_path_step` (L4245).
- Read (retained for migration): `check_path_ids_exist` (L3748),
  `get_all_paths` (L3789), `get_path_with_steps` (L3892),
  `get_path_overview` (L3962), `get_path_full` (L4149), plus chapter/step
  read variants (L4063, L4147, L4253, L4516).

Surfaces still wired to the deprecated WRITE functions:

- `elohim/sdk/src/client/zome-client.ts` — `createPath`, `addPathStep`,
  `updatePath`, `updateStep`, `batchAddPathSteps` zome-dispatch wrappers
  (L228–~L275).
- `elohim/sdk/src/services/path.service.ts` — `PathService.create` /
  `.addStep` delegate to the above (L36, L48, L64, L84). NO live consumer
  calls the SDK *write* surface — only the JSDoc example in
  `elohim/sdk/src/index.ts` (L36–37) references `sdk.paths.createSimple` /
  `addSteps`. The Angular app's `PathService`
  (`app/lamad/src/app/services/path.service.ts`) is a SEPARATE, read-oriented
  service and is not affected.
- `genesis/seeder/src/seed-production.ts` (L477, L500) and
  `genesis/seeder/src/add-elohim-steps.ts` (L127) — ORPHANED scripts: not in
  `genesis/seeder/package.json` scripts and not imported anywhere. They would
  fail at runtime (write functions error) if invoked. The live seeder
  (`seed.ts` / `seed-sqlite.ts`) already uses `create_content` +
  `bulk_create_content` with `contentType: 'path'`, `contentFormat:
  'epr-composite'` (seed.ts `pathToContent`, L1662).
- `elohim/elohim-agent/mcp-servers/elohim-content/` `create_path` tool —
  NOT affected: it writes to a local file DATA_DIR (`tools/path-tools.ts`
  `createPath`), not the zome dispatch.

## Migration path

The canonical replacement is already live and load-bearing:
author paths as `Content` (`create_content` / `bulk_create_content`) with
`content_type = "path"`, `content_format = "epr-composite"`, steps modeled as
EPR sections + `Relationship` entries. See `seed.ts pathToContent` and
`genesis/docs/superpowers/specs/2026-06-08-native-content-graph-seam-design.md`.

Full retirement (the remaining debt) means:
1. Delete the deprecated WRITE coordinator functions from the HDK dispatch
   table in `content_store/src/lib.rs`, OR formally keep them as permanent
   error-return stubs (decision needed).
2. Confirm no in-DHT migration still depends on the retained READ functions,
   then decide their fate (keep for legacy DHT reads vs remove).
3. Remove the now-dead SDK write methods (`zome-client.ts`,
   `path.service.ts`) and the matching `CreatePathInput` / `AddPathStepInput`
   ts-rs-derived types — and regenerate bindings.
4. Delete the orphaned seeder scripts `seed-production.ts` and
   `add-elohim-steps.ts` (or rewrite them onto `create_content`).

## Current decision

BLOCKED for background auto-fix — canonicalized, not fixed.

This is an intentional, in-progress architectural migration (paths→Content,
M-REA-1 era; the deprecation block was last touched in 000e144f7), and the
deprecated surface is deliberately retained for migration reads. Retiring it
crosses the DNA HDK dispatch table + ts-rs cross-crate binding boundary
(removing `CreatePathInput`/`AddPathStepInput` regenerates `export_bindings`
and ripples into every referencing `.ts`) and requires a substrate-side
confirmation that no live DHT migration still reads via the retained read
functions. That exceeds bounded-fix posture (DNA dispatch + ts-rs migration =
operator-initiated rust-architect sprint, not a background change). The
orphaned seeder scripts could be deleted cheaply, but they are harmless
(unreferenced) and belong to the same retirement unit — splitting them off
would leave the concern open anyway, so they ride with the sprint.

No action lands this run beyond canonicalization. The sentinel cites THIS
decision deterministically on every re-encounter of the six fingerprints; the
stasis sweep re-checks when the paths→Content migration sprint is scheduled.

## Verification

N/A — blocked (no fix applied). The deprecated code compiles clean today;
nothing is broken. Closure of this concern happens when the retirement sprint
lands and deletes the deprecated surface (at which point the ledger lines are
removed and this entry is deleted, the commit being the record).
