---
id: "backlog-edge-manifest-transitive-crate-source-watch-gap"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Edge build-manifest doesn't watch transitive baked-crate sources (elohim-render/**, elohim-chrome-asset/**) — a source-only change rebuilds neither image"
slug: "edge-manifest-transitive-crate-source-watch-gap"
written: "2026-06-27"
author: "shake-out-dev-pipeline shift (2026-06-26T22-00)"
status: "backlog"
priority: "medium"
jobs: [elohim-edge]
---

## The gap

`elohim/holochain/build-manifest.json` declares the edge pipeline's change-detection
`sources` per task. Both image-build tasks watch only their OWN crate's tree:

- `cargo-build-doorway.inputs.sources`: `doorway/doorway-service/{src,Cargo.toml,Cargo.lock}`, `doorway-client/**`, and (added `f3c1b34dd`) `doorway/doorway-service/Dockerfile`.
- `cargo-build-storage.inputs.sources`: `elohim/elohim-storage/{src,benches,migrations,Cargo.toml,Cargo.lock,Dockerfile}`, `elohim/sdk/**`.

But BOTH images **compile transitive path-dep crates inside Docker** — `elohim-render`
and (via render) `elohim-chrome-asset`, plus the other crates COPYd into each build
context. Neither task watches `elohim/elohim-render/**` or `elohim/elohim-chrome-asset/**`.

**Consequence:** a SOURCE-ONLY change to elohim-render or elohim-chrome-asset (no
version bump → no consuming-crate `Cargo.lock` delta) triggers NO rebuild of either
the doorway or storage image. The orchestrator routes it to nothing, and the deployed
images silently carry stale baked crate code until some unrelated change forces a
rebuild. This is the same principle-7 change-detection class that hid the doorway
Dockerfile fix for an hour during the 2026-06-26 chrome-asset shake-out (that narrower
Dockerfile-watch gap is now fixed; this source-watch gap remains).

## Proposed fix

Add the transitive baked-crate source globs to both tasks' `sources`. Minimum:
`elohim/elohim-render/**`, `elohim/elohim-chrome-asset/**`. Consider a complete audit
of every crate COPYd into each Dockerfile build context and watch all of them (the
doorway Dockerfile COPYs render, peer-fabric, …; storage COPYs render, constitution,
epr, compute, …) so any baked-crate change rebuilds the consuming image. Adding
watch-paths only ever makes the orchestrator rebuild MORE (accurately), never less.

## Evidence / refs

- Shift journal: `.claude/shifts/2026-06-26T22-00-shake-out-dev-pipeline.journal.md` (iter-10).
- Memory: `project_new_path_dep_needs_dockerfile_copy` (transitive-dep + manifest-watch variant).
- The narrower Dockerfile-watch fix that surfaced this: commit `f3c1b34dd`.
- Dep chain proof: `cargo tree -i elohim-chrome-asset` → chrome-asset ← elohim-render ← doorway / elohim-storage.
