---
title: constants-sync pre-push gate ENOENTs in any fresh worktree — lens-market-view.ts gitignored while sibling views are tracked
created: 2026-07-17
status: OPEN
domain: D-ci
source: overnight shift p2p-dataplane-resiliency-convergence (batch-1 push, 2026-07-17)
severity: low
---

`elohim/sdk/schemas/.gitignore:1` ignores the whole `generated-ts/` tree, but every
view under `generated-ts/views/` EXCEPT `lens-market-view.ts` was historically
force-added and is tracked. A fresh clone/worktree therefore checks out all sibling
views but not `lens-market-view.ts`, and the pre-push `constants-sync` gate
(`codegen-ts.mjs -- --verify`) dies with ENOENT reading it — failing ANY push from a
fresh worktree regardless of the diff (observed on a seeder-only change; the main
dev checkout masks the bug because the file exists there from prior codegen runs).

Fix options (pick one):
1. `git add -f elohim/sdk/schemas/generated-ts/views/lens-market-view.ts` — make it
   consistent with its tracked siblings (preferred; one commit).
2. Teach `codegen-ts.mjs --verify` to generate-if-missing instead of ENOENT.

Workaround used tonight: run `pnpm run schema:codegen:ts` (generate mode) in the
worktree before pushing — creates the file, no tracked churn.
