---
name: project_doorway_ab_edge_islanding
description: doorway A/B are two independent edges over the alpha genesis pair (matthew/adam), island-by-construction with no cross-edge coherence; the e0352a7/8a2c65e glyphs were buildIds not content CIDs
metadata:
  type: project
---

Two doorways, **NOT load-balanced**: `doorway-alpha.elohim.host` → doorway-alpha → **matthew** conductor (on-prem household, genesis peer), MongoDB `doorway-alpha`, doorwayId `alpha-elohim-host`; `elohim.host` (apex) → doorway-alpha-b → **adam** conductor (shem/remote, genesis peer), MongoDB `doorway-alpha-b`, doorwayId `apex-elohim-host`. nginx ingress uses `upstream-hash-by $binary_remote_addr` (client-IP sticky), **no LB, no CDN, no shared cache**; the apex has no fallback to matthew.

**Each edge is an ISLAND BY CONSTRUCTION:** per-pod in-memory kitsune2 bootstrap store (DashMap, fresh per boot, no cross-replica sync), separate MongoDB projection cache, independent per-pod EPR router (fetches only its own `STORAGE_URL`). doorway = CDN-edge projection, not a consensus node → cross-edge EPR-head coherence depends ENTIRELY on the matthew↔adam conductor DHT gossip; a partition (DNA-hash skew / bootstrap islanding) or one edge down ⇒ the two hostnames serve different heads with **no detection**.

**GOTCHA:** the "two EPR heads" `e0352a7` / `8a2c65e` are deployed **git build SHAs** (`servingContext.buildId`), NOT content CIDs (those are `bafy…`). The symptom was **deploy-version skew** between the two edges + matthew crash-looping (surfacing different builds across restarts). Deploy is sequential per-deployment rollout with no atomic A+B gate (version skew possible); genesis-pair DNA-reinstall-flag coherence is operator-discipline, not enforced (CLAUDE.md partition warning).

**Live 2026-06-14:** elohim.host/adam HEALTHY (peerCount 13, head `bafyreigrmzvo5h…`, content `sha256-1a76…`); doorway-alpha/matthew DOWN (nginx 503, crash-loop). Snapshot at-risk/zeros are **honest/MEASURED** (real placement gap: 1 steward requested, 0 achieved) via `/api/v1/resilience/{id}/household`; the legacy `/api/v1/resilience/{id}` is the unmeasured-default trap (zeros for any id). `/api/v1/federation/p2p-peers` under-reports (total:1 self vs peerCount 13 = read-model wiring gap).

**Architectural verdict:** cross-edge coherence belongs at the substrate (matthew↔adam gossip + shared/persistent bootstrap), not the doorway edge; the edge needs at minimum a divergence DETECTOR (today the A/B split is invisible until a human notices the build glyph). Plans: `genesis/docs/superpowers/plans/2026-06-14-federation-*`. Related [[project_doorway_kitsune2_bootstrap_protocol]] [[project_alpha_topology_bootstrap_pair]] [[project_collective_topology_author_stewards]].
