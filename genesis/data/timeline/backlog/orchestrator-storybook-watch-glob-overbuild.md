---
id: "backlog-orchestrator-storybook-watch-glob-overbuild"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "elohim-storybook dispatches on docs-only architecture/*.md changes — over-broad watch glob"
slug: "orchestrator-storybook-watch-glob-overbuild"
written: "2026-08-06"
author: "agentic-developer"
status: "open"
priority: "low"
area: "ci"
domain: "code"
tags: [orchestrator, graph-walker, watch-globs, over-build, code-domain]
---

# elohim-storybook over-builds on docs-only changes

Observed 2026-08-06 (orchestrator #1624, push 728829dfc): a batch touching only
`genesis/data/timeline/backlog/*.md`, `genesis/docs/content/elohim-protocol/architecture/*.md`,
and `genesis/orchestrator/manifests/edgenode/alpha.yaml` dispatched **elohim-storybook** —
graph-walker attributed it to the architecture `.md` file. A pattern-library build has no
dependency on protocol architecture docs; the storybook build-manifest watch glob is too broad
(likely a bare `genesis/docs/**` or similar). Cost: a wasted storybook build per docs push,
eating executor time during integration windows (principle-7 class: silent over-build).

Fix shape: narrow the offending glob in the storybook project's `build-manifest.json`
(app/elohim-library/build-manifest.json or wherever `elohim-storybook` is declared) to the
design-guide sources it actually renders; verify with
`git diff --name-only <docs-only-range> | node genesis/orchestrator/graph-walker.mjs` → no storybook.

## 2026-08-25 recurrence — `CLAUDE.md` routed as SOURCE hits app AND storybook

Orchestrator #1722 (push 286089679, 88 files): the only app-tree changes were cite-status
annotations in `app/elohim-elements/CLAUDE.md` and `app/elohim-library/CLAUDE.md`, yet the
PER-FILE ROUTING MATRIX routed them `→ elohim:build-angular, elohim:lint-library` and
`→ elohim-storybook:build-storybook`. Cost this time: a 58-min app build (#1670) and a storybook
build for zero code change, both racing the real edge landing for executors. So the class is
wider than the storybook glob: any per-project `build-manifest.json` whose `sources` admit the
gospel docs treats a CLAUDE.md/AGENTS.md edit as a code change. Fix shape (one place, not per
manifest): exclude `**/CLAUDE.md`, `**/AGENTS.md`, `**/*.md` from source matching in
`genesis/orchestrator/graph-walker.mjs` unless a project opts a docs glob in explicitly; verify
with `git diff --name-only a5607e938..286089679 -- 'app/**/CLAUDE.md' | node genesis/orchestrator/graph-walker.mjs`
→ no app, no storybook. Not applied mid-landing (shift 2026-08-25T0210: an orchestrator change
would have re-dispatched and muddied attribution).

## 2026-09-01 recurrence — unmapped dataplane stories trigger Storybook

Orchestrator #1768/#1769/#1770 routed
`genesis/a2o/features/dataplane/coordinator-hot-swap.feature` and
`genesis/a2o/features/dataplane/conductor-bridge-recovery.feature` into Storybook builds
#298/#299/#300 through the blanket `genesis/a2o/features/**` input. Storybook's Genesis sync
imports an explicit mapping set and does not import `dataplane/`, so these dispatches could not
change the rendered artifact.

The recurrence is fixed by replacing that blanket input with the exact feature globs exported by
`scripts/sync-genesis.mjs`. A parity test now requires the build-manifest feature watches to equal
the sync mappings, preventing either side from drifting independently. Graph-walker verification:

- `features/dataplane/coordinator-hot-swap.feature` → no `elohim-storybook` project or pipeline.
- mapped `features/lms/`, `features/rms/`, and future `features/wms/` stories →
  `elohim-storybook` remains selected.

The backlog item stays open for the distinct 2026-08-25 app-gospel recurrence above.
