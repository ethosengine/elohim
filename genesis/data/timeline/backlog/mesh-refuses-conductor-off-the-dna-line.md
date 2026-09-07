---
id: "backlog-mesh-refuses-conductor-off-the-dna-line"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "hc-mesh.sh refuses a conductor whose line differs from the DNA's hdk line and auto-detects the pinned fork build — a stock 0.6 conductor was only warned about while the start proceeded to three dead cells"
slug: "mesh-refuses-conductor-off-the-dna-line"
written: "2026-09-07"
author: "orchestrator (second-opinion session 2026-09-06/07)"
status: "triaged"
priority: "medium"
jobs: [elohim-app]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-workspace-image-ships-stock-holochain-0-6"
  - "habit:dataplane-convergence"
tags: [local-mesh, toolchain, holochain-0.7, fail-closed]
---

**What bit (2026-09-06/07):** with the workspace image on stock 0.6.0 (sibling atom), `just mesh start` in a
fresh shell resolved `command -v holochain`, `assert_toolchain_parity` passed (hc and holochain were both 0.6.0),
and the only signal was the status banner "STOCK — alpha runs the fork, so this mesh is NOT at parity". The
hdk-0.7 hApp cannot install on a 0.6 conductor, so the Prologue failed far from the cause. The slice-1
(accountable-correction) mesh run hit this twice across restarts.

**Landed (2026-09-07, `app/elohim-app/scripts/hc-mesh.sh`):**
- `MESH_FORK_BIN_DIRS` now ends with `$MESH_TOOLS_DIR/hc-fork-<pin12>/bin`, where `<pin12>` is read from the
  `elohim/holochain-conductor` gitlink (`git ls-tree HEAD`, never a checkout — the submodule is `update = none`).
  A conductor bump changes the directory name, so a stale fork build can never be picked up silently.
- `assert_conductor_matches_dna`: parses `hdk = "=X.Y.Z"` from `elohim/holochain/dna/elohim/Cargo.toml` and
  refuses a conductor whose `X.Y` differs, naming both and the two places a matching build lives.
  `MESH_ALLOW_TOOLCHAIN_SKEW=1` overrides deliberately, as for the existing hc/holochain pair check.
- The status banner now says the start is REFUSED unless the line matches, instead of "NOT at parity".

**Verified:** `just mesh status` → `NEXT LAUNCH: …/tools/hc-fork-25dd2d0be144/bin/holochain (holochain 0.7.0)
[FORK, auto-detected]` with no `HOLOCHAIN_BIN` set; stock 0.6.0 resolves to line 0.6 ≠ DNA 0.7.

**Retire when:** the workspace image ships 0.7 AND the mesh defaults to the fork by pin — then the refusal is a
regression guard only and this atom decomposes into the mesh runbook.
