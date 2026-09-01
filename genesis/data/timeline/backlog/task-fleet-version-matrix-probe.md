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
claimedBy: "codex"
relatedNodeIds:
  - "backlog-task-runtime-passport-endpoint"
  - "backlog-upgrade-propagation-p2p-design-arc"
tags: [observability, a2o, mixed-version, delegable]
---

**Claimable by any implementation agent. New-file-only; zero collision.**

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

- The delegated implementation agent (Codex or equivalent) MAY create
  `genesis/a2o/scripts/version-matrix.ts` only.
- It MUST NOT edit package scripts or any existing script; `http.rs` (including
  the `/version` match arm); `happ_manager.rs`; any Jenkinsfile; any
  deployment/orchestrator manifest; `hc-mesh.sh`;
  `src/p2p/view_federation.rs`; or any Rust source. Those are the rung lane's
  surfaces this week.

## DoD + verification

- Against the running local mesh: `cd genesis/a2o && pnpm exec tsx scripts/version-matrix.ts` renders 3 peer columns with build info; with conductor ports supplied it adds the coordinator row (expect `coordswap-rung1-proof` marker on all three).
- Unreachable-peer path proven (point one entry at a dead port → exit 2, table shows the gap).

## Implementation + verification status (2026-09-01)

The script is implemented in `60850aa72`; `fbcdedee8` later added the
orthogonal `--observed` rendering without changing the default matrix. Focused
ESLint, Prettier, and isolated strict TypeScript checks pass. A three-server
localhost fixture proved mixed nested/legacy `/version` shapes, `DIVERGENT`
marking, `--json`, environment input, and the dead-peer path (exit 2 with an
explicit gap).

At `2026-09-01T19:02Z`, a live three-peer dual mesh answered the default probe
with exit 0: matthew, jessica, and james each rendered the runtime passport and
reported `elohim-storage/0.1.0`. The optional conductor leg remains unproven.
Both this script and the known-good `coordswap-probe-build-info.ts` found
`zome_build_info` absent from the freshly installed coordinator. Applying the
previously proven `coordswap-rung1-proof` bundle then failed closed on matthew
and did not advance to the other peers: the conductor returned
`DbConnectionPoolError` opening its local wasm database, and the rolling
driver's re-check correctly reported that drift remained.

Follow-up diagnosis named the blocker **orphaned-live-conductor / unlinked
data-root**, not bundle lineage and not an embedded/external endpoint-routing
error. The `?apply=false` report read the same installed truth as `/version`
(`get_dna_definition`) and showed the expected coordinator-only Lamad drift:
installed `content_store = uhCokJ38rRzUyb_lejmSZVryqTqJ8xqccMhErjIMB22210eSKRcNd`,
bundled `content_store = uhCokdzG4oJduMN074ZDZ8VPOKrGIe9OCH_UH5iUAFFApzdeIr6vr`.
Unpacking the exact 10,308,450-byte proof bundle confirmed that all five DNA
hashes match the installed/local hApp; only `lamad.dna` differs, so the lineage
guard correctly allowed the attempt. The apply call reached
`update_coordinators`, which then failed opening
`local-dev/matthew/databases/wasm/wasm`. The same run's sandbox log records a
fresh generation failing with `PermissionDenied`, while the storage process
subsequently connected to an older conductor still answering on ports 4444/4445
and immediately reported missing DHT database paths. The sandbox data-root
directory was absent after shutdown. A corrected coordinator invocation cannot
be proved until the mesh lifecycle refuses to delete/regenerate a sandbox while
any matching conductor process still survives (or otherwise verifies the data
root before declaring that conductor reusable); the guard must remain strict.

Therefore the implementation is materially present, but this atom stays
`open`: no receipt yet shows `zomeBuildInfo.buildMarker =
coordswap-rung1-proof` on all three live conductors, and no full alpha storage
version matrix has been captured.
