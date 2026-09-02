---
id: "backlog-app-generated-manifest-types-stale-vs-codegen"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "elohim-app imagodei + shefa generated files are stale against their codegen — regen drops an onConsume coupling block, so `just codegen all verify` (and the pre-push codegen leg) is red on dev"
slug: "app-generated-manifest-types-stale-vs-codegen"
written: "2026-09-02"
author: "session-2026-09-01-adoption-ceremony"
status: "open"
priority: "medium"
jobs: [elohim-app]
cluster: "arch-frontend-bundle-seams-backlog"
relatedNodeIds:
  - "backlog-elohim-app-gate-lint-debt-blocks-push"
tags: [codegen, elohim-app, gate-debt, pre-push]
---

## Measured (2026-09-02 01:2xZ, dev at 1faf1de03)

`just codegen all verify` → FAIL for `app/elohim-app/src/app/imagodei/generated/{manifest-types,
coupling-map}.ts` and `app/elohim-app/src/app/shefa/generated/{metadata-types,manifest-types,
coupling-map}.ts`. Neither domain manifest changed in the 36-commit rung-5 batch (imagodei
manifest last touched 54a4b482c 2026-07-17), so this is standing dev debt, not batch drift.

`pnpm run imagodei:codegen` is a stable fixed point (same diff on repeated runs) but the diff is
NOT cosmetic: besides Prettier-style differences (wrapped signatures, quoted keys) the regenerated
`coupling-map.ts` DROPS the `role.onConsume { action: 'use', resourceConformsTo:
'capability-grant', recognition: 'role-record' }` block the committed file carries. Either the
generator regressed (lost onConsume emission) or the committed file was hand-edited past the
manifest. Committing the regen blindly would silently remove a coupling.

## Fix

Diff the manifest's role entity against the generator's coupling emission; restore onConsume
emission (or move the coupling into the manifest if it was hand-authored), regenerate both
domains, verify `just codegen all verify` exits 0 and the app gate stays green.
