---
id: "backlog-task-fleet-version-matrix-probe"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Task: fleet version-matrix probe — one command that renders who runs what across mesh or fleet peers"
slug: "task-fleet-version-matrix-probe"
written: "2026-08-31"
author: "session-2026-08-31-velocity-snowball"
status: "open"
priority: "medium"
jobs: [elohim-genesis]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-task-runtime-passport-endpoint"
  - "backlog-upgrade-propagation-p2p-design-arc"
tags: [observability, a2o, mixed-version, delegable, codex-suitable]
---

**Claimable by any agent (Codex-suitable). New-file-only; zero collision.**

## Why

Mixed-version peers are the upgrade arc's steady state. Debugging needs
"the version matrix" as one table, not seven hand-curled endpoints: which
storage build, which conductor, which coordinator wasm, which kernel —
per peer, side by side, with disagreements highlighted.

## Scope

New script `genesis/a2o/scripts/version-matrix.ts` (tsx, patterned on
`coordswap-probe-build-info.ts` / `look.ts` conventions):

1. Input: peer list as `name=http://host:port` CSV via argv or
   `PEER_STORAGE_URLS` env (mesh default: matthew/jessica/james on
   localhost:8090-8092). Optional `name=admin:app` conductor ports CSV to
   also call `zome_build_info` (mesh only; skip silently when absent).
2. For each peer: `GET /version` (the runtime passport when landed — but
   the script must work TODAY against the current BuildInfo-only shape:
   render what's there, dash what's missing), plus optional
   `zome_build_info` zome call.
3. Output: a human table (rows = fields, columns = peers) with a
   `DIVERGENT` marker on any row where peers disagree, and `--json` for
   machine use. Exit 0 always when all peers answered; exit 2 if any peer
   was unreachable.
4. Write nothing to disk except with an explicit `--out <path>`.

## Disjointness contract

- MAY create: `genesis/a2o/scripts/version-matrix.ts` only.
- MUST NOT touch: hc-mesh.sh, existing scripts, any Rust, Jenkinsfiles,
  manifests.

## DoD + verification

- Against the running local mesh: `cd genesis/a2o && pnpm exec tsx scripts/version-matrix.ts` renders 3 peer columns with build info; with conductor ports supplied it adds the coordinator row (expect `coordswap-rung1-proof` marker on all three).
- Unreachable-peer path proven (point one entry at a dead port → exit 2, table shows the gap).
