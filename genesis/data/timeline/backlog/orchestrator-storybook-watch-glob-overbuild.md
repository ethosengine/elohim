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
