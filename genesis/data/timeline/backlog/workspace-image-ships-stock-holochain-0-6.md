---
id: "backlog-workspace-image-ships-stock-holochain-0-6"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Workspace image still ships stock holochain 0.6.0 — the rust-dev Dockerfile pins 0.7.0 since 2026-09-03 but Harbor :latest predates it; rebuild + republish, then every workspace restart lands on 0.7"
slug: "workspace-image-ships-stock-holochain-0-6"
written: "2026-09-07"
author: "orchestrator (second-opinion session 2026-09-06/07)"
status: "open"
priority: "high"
jobs: [che-devworkspaces]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-mesh-refuses-conductor-off-the-dna-line"
  - "habit:dataplane-convergence"
tags: [devspace, image, holochain-0.7, toolchain, ci-hygiene]
---

**Evidence (2026-09-07 00:4xZ, after the box reboot):** `/opt/holochain/bin/{holochain,hc,hcterm}` in the
running workspace are `holochain 0.6.0`, dated 2026-08-26. The image recipe
`che-devworkspaces/containers/rust-dev/Dockerfile:157` has `ARG HOLOCHAIN_VERSION=0.7.0` since commit
`5d7e482` (2026-09-03). The devfile pulls `harbor.ethosengine.com/devspaces/udi-plus-mem-rust-nix:latest`
(`devfile.yaml:90`); that tag was built before the pin moved, so every restart (including today's reboot)
re-lands on 0.6.0. The 0.7 binaries only exist under `/projects/.claude-config/tools/{hc-0.7,hc-fork-25dd2d0be144}`,
which is why `just mesh status` kept printing "STOCK — NOT at parity" and why mesh runs only worked with
`HOLOCHAIN_BIN` set by hand.

**Cure (operator-owned — an image pipeline run, not a repo edit):** rebuild and publish the rust-dev image so
`:latest` carries 0.7.0 (and `hc`/`hcterm` from the same release), then restart the workspace. Consider pinning
the devfile to a versioned tag instead of `:latest` so an image drift is visible in git.

**Interim (landed 2026-09-07 in `app/elohim-app/scripts/hc-mesh.sh`):** the mesh auto-detects the pinned fork
build at `$MESH_TOOLS_DIR/hc-fork-<submodule-pin12>/bin` and refuses a conductor whose line differs from the DNA's
hdk line — see the sibling atom.

**Done when:** a fresh workspace prints `holochain 0.7.0` for `/opt/holochain/bin/holochain`, and `just mesh
status` shows the fork as NEXT LAUNCH with no `HOLOCHAIN_BIN` in the environment.
