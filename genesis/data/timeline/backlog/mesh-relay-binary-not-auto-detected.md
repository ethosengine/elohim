---
id: "backlog-mesh-relay-binary-not-auto-detected"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "hc-mesh.sh auto-detects the fork conductor from the tools dir but finds iroh-relay only via MESH_RELAY_BIN or PATH — a fresh shell refuses `just mesh start` with 'no iroh-relay binary' while /projects/.claude-config/tools/iroh-relay-1.0.3/bin/iroh-relay sits next to the fork"
slug: "mesh-relay-binary-not-auto-detected"
written: "2026-09-07"
author: "overnight shift 2026-09-07"
status: "open"
priority: "medium"
jobs: [elohim-app]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-mesh-refuses-conductor-off-the-dna-line"
tags: [mesh, hc-mesh.sh, iroh-relay, dev-loop]
---

**Evidence (2026-09-07 09:0xZ):** `just mesh start` from a fresh workspace shell exited 1: "no iroh-relay binary (MESH_RELAY_BIN unset and none on PATH)". The binary exists at `/projects/.claude-config/tools/iroh-relay-1.0.3/bin/iroh-relay` (the same `MESH_TOOLS_DIR` the conductor auto-detection reads). Cure: extend the resolver that finds `hc-fork-<pin12>/bin` to also try `$MESH_TOOLS_DIR/iroh-relay-*/bin/iroh-relay` (newest version wins) before refusing. **Done when:** a fresh shell with no MESH_* env starts the mesh and `just mesh status` names the relay binary and version.
