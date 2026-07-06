---
id: "backlog-eprfs-fuse-mount-lazy-materializer"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "eprfs-fuse — a lazy FUSE-mount materializer sibling to LocalMaterializer"
slug: "eprfs-fuse-mount-lazy-materializer"
written: "2026-07-06"
author: "eprfs-agent capability-projection V2 plan (Task 8 backlog capture)"
status: "backlog"
priority: "low"
jobs: [elohim]
tags: [eprfs, eprfs-local, eprfs-fuse, fuse, materializer, lazy-hydration, projection-manifest]
cites:
  - genesis/docs/superpowers/plans/2026-07-06-eprfs-agent-capability-projection-v2-plan.md
  - elohim/eprfs/eprfs-local/src/lib.rs
  - elohim/eprfs/eprfs-core/src/projection.rs
---

## What

`eprfs-local`'s `LocalMaterializer` already models on-demand hydration as data, not code, via
`MaterializationPolicy::{Sparse, FetchMissing, LocalOnly}` (`elohim/eprfs/eprfs-local/src/lib.rs`).
`Sparse` writes placeholder/stub entries without fetching blob bytes; `FetchMissing` fetches on
demand when an entry is materialized but not yet present locally. That is, eagerly, exactly the
policy shape a FUSE filesystem needs: don't touch the network/storage backend until a path is
actually `read()`.

A new `eprfs-fuse` crate would implement a FUSE filesystem backend that satisfies reads by
resolving into the SAME `ProjectionManifest` (`elohim/eprfs/eprfs-core/src/projection.rs`) that
`LocalMaterializer` walks — i.e. it is a second *materializer*, not a second *projector*. Because
V2's whole architecture separates "what gets projected" (the `ProjectionManifest`, produced once by
`project()`) from "how it lands on disk" (the materializer), FUSE is a drop-in alternate
materializer: no change to `eprfs-agent`, `eprfs-core`'s `project()`, or any domain adapter is
required to add it.

## Why this matters

Today, using a projected capability tree (e.g. a `.claude/agents/*` surface projected from
canonical `agent` EPRs) means running `LocalMaterializer::materialize()` to write every entry to
disk eagerly. For large capability trees (or once V2 scales to `skill`/`agent-spec`/`hook` per the
sibling backlog item `eprfs-agent-scale-skill-agentspec-hook-waves.md`), a lazy FUSE mount avoids
paying full materialization cost just to make one file visible to a tool that opens it — the same
sparse/on-demand tradeoff `MaterializationPolicy::Sparse`/`FetchMissing` already made a first-class
concept for the eager path.

## Design direction (not built here)

- `eprfs-fuse` depends on `eprfs-core` (for `ProjectionManifest`, `EprfsStorage`) and a FUSE binding
  crate (e.g. `fuser`), sibling to `eprfs-local` in the `elohim/eprfs` workspace.
- Its `read()`/`readdir()`/`getattr()` handlers resolve against the manifest's entries directly;
  blob bytes are fetched from the backing `EprfsStorage` lazily, gated by the same
  `MaterializationPolicy` enum `LocalMaterializer` already consumes (reused, not reinvented).
- Drift/verify (`verify_projection` in `eprfs-local`) is orthogonal — a FUSE mount that always
  resolves against the live manifest has no drift by construction (there's no on-disk copy to go
  stale); this backlog item does not change that primitive.

## Blocked on

Nothing. Sequence after the scale waves (`eprfs-agent-scale-skill-agentspec-hook-waves.md`) so FUSE
serves a real multi-class capability tree rather than only `agent`, but there is no structural
blocker — it could be built against the `agent`-only V2 surface today if prioritized.

## Provenance

Surfaced by `genesis/docs/superpowers/plans/2026-07-06-eprfs-agent-capability-projection-v2-plan.md`
Task 8 (Step 3: FUSE is the lazy sibling of `LocalMaterializer`, sequenced after the scale waves).
