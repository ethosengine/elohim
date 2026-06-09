---
id: "backlog-ci-storybook-elohim-elements-dts-build-order"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Storybook build emits non-fatal TS2307 on @elohim/storage-client/generated — elohim-elements DTS phase runs before storage-client dist/generated is built"
slug: "ci-storybook-elohim-elements-dts-build-order"
written: "2026-06-09"
author: "claude-opus-pipeline-shakeout"
status: "documented"
priority: "low"
# Surfaced while triaging elohim-storybook/dev #148 (whose actual FAILURE was an
# unrelated infra pod-eviction flake, tracked in ci-findings ledger 14f926653a27).
# During the "Build elohim-elements" stage, app/elohim-elements/elohim-core emits:
#   src/elohim-compute-tile.ts:15:8 - error TS2307: Cannot find module
#   '@elohim/storage-client/generated' or its corresponding type declarations.
# This is NON-FATAL — it appears only in the [vite:dts] declaration phase; the
# bundle builds (`✓ built`) and register.js is produced. The package export map
# IS correct (elohim/sdk/storage-client-ts/package.json declares the "./generated"
# subpath → dist/generated/index.d.ts). The error is a build-ORDERING artifact:
# the elohim-elements DTS phase resolves types before the storage-client SDK's
# dist/generated/ has been (re)built, so the .d.ts isn't on disk yet. Co-occurred
# with the ContentGraph* codegen commits that added new generated types.
ci_status: documented
jobs: [elohim-storybook]
relatedNodeIds: []
tags: [ci, elohim-storybook, elohim-elements, storage-client, build-order, dts, non-fatal, low-priority]
cites:
  - https://jenkins.ethosengine.com/job/elohim-storybook/job/dev/148/
  - app/elohim-elements/elohim-core/src/elohim-compute-tile.ts
  - elohim/sdk/storage-client-ts/package.json
---

# CI: Storybook elohim-elements DTS resolves `@elohim/storage-client/generated` before it is built

## What

The `elohim-storybook` pipeline's "Build elohim-elements" stage emits a TS2307 in
`app/elohim-elements/elohim-core/src/elohim-compute-tile.ts:15` for the
`@elohim/storage-client/generated` type import. It is **non-fatal** (declaration-file
phase only; the vite bundle builds successfully) and did **not** cause #148's FAILURE
(that was an infra pod-eviction flake).

## Why it matters

A non-fatal type-resolution warning that depends on build ordering is fragile — if a
future change makes the elohim-elements DTS phase strict, or the storage-client dist
is absent rather than stale, this flips fatal. The export map is already correct, so
the durable fix is ordering/dependency, not packaging.

## Proposed fix

Ensure the storybook prebuild builds (or `tsc`-emits the `.d.ts` for) the
`@elohim/storage-client` SDK's `dist/generated/` **before** the elohim-elements DTS
phase runs — e.g. a build-order dependency in the storybook Jenkinsfile / the
`app/elohim-library` prebuild script, or a project-reference so the elements build
waits on the SDK types. Verify by confirming the TS2307 no longer appears in a clean
storybook build.

## Status

`documented` — not actioned. Discovered during the 2026-06-09 pipeline-shakeout shift
(primary deliverable: the elohim app Unit Test zone-rejection fix). Low priority;
non-blocking. Pick up as a standalone storybook-pipeline build-order task.
