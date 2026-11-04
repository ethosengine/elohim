---
id: "backlog-native-dev-track-a-eprfs-core-slice4-derivation-plane"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Track A: land eprfs-core slice 4 (walk_spine, project_view, derive_or_reuse) and retire ark's identity-projection workaround"
slug: "native-dev-track-a-eprfs-core-slice4-derivation-plane"
written: "2026-11-04"
author: "claude (design pass, native-dev-plane spec; steward human:matthew)"
status: "backlog"
priority: "high"
relatedNodeIds:
  - "native-dev-plane-one-derivation-earned-reach-design"
tags: [native-dev-plane, derivation, eprfs-core, ark, foundational, track-a]
---

Source spec: `native-dev-plane-one-derivation-earned-reach-design` (genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md).

## Item (spec §11 Track A, ruling D2)

Write-set: `elohim/eprfs/eprfs-core`. Land slice 4 of placement-contract-interfaces §C: `walk_spine`, `SpineWalk`, `project_view`, `derive_or_reuse`, the `P` vectors, and `tests/derive.rs`. Then delete ark's identity-projection workaround.

**Verified 2026-11-04.** `elohim/ark/cli/src/material.rs:101-113` is `projected()`; its doc comment says "`project_view` has not landed in eprfs-core" and it refuses any view other than one naming every closure field. `grep walk_spine` over this backlog finds nothing, so no existing item covers this.

**Why foundational.** One object is both the cache key and the sparse checkout (D2); every join (F1r, F2, F4) keys on it. Operator steer 2026-11-04: no interim key.

**Receipt.** `P` vectors green; ark keys a task on a real projected root. Expected wall-clock removed: none alone; it unblocks every join.

**retire-when.** The `projected()` identity arm in `material.rs` is gone and ark calls `project_view`.

**Open (spec §13).** Projected-view size census for non-Rust closures (pnpm workspace, DNA in-tree `./target`, unlocked flake inputs) once `project_view` lands.
