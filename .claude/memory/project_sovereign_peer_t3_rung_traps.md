---
name: project_sovereign_peer_t3_rung_traps
title: Sovereign-peer T3 rung traps
description: "Stock tx5 conductor is listed-but-unconnected to the iroh fleet; alpha agent-infos advertise storageArc null so a joiner's reads miss in both arc modes."
metadata:
  type: project
---

The T3 hybrid rung is `just dev conductor alpha` (fork iroh pair from the cargo pool; `hc-start.sh` refuses a stock join unless `ALLOW_STOCK_JOIN=1`). Two traps measured 2026-08-28 (M0 shift, spec ratchet-to-delivery-dataplane-sdk-lanes, lane P rung P5):

- **Listed ≠ joined.** A stock holochain 0.6.0 (tx5) conductor publishes to alpha's bootstrap and is listed by `conductor-diagnostics`, then holds `connections: []` forever — alpha's conductors are iroh. The 08-28 sovereign-peer spike was this; its 404 was over-determined.
- **Arc-null read void.** All 27 live alpha agent-infos carry `storageArc: null`. A full-arc joiner (`target_arc_factor 1`) is its own authority and reads its empty cache until gossip fills it (hours); a zero-arc joiner has no authority to ask and misses in 7 ms. `sovereign-peer-join.feature` sc1 is green on the fork; sc5 stays red until the fleet advertises arcs — backlog `sovereign-peer-network-read-no-authorities`.

**Why:** two probes ("listed", "connected") both pass on a peer that can neither read nor be read; only peer-store-holds-others + `dumpNetworkStats.connections` + a real read falsify it.
**How to apply:** never claim a join from diagnostics alone; kitsune2 `space`/`agent` cores are base64url of bytes 3..35 (never slice the base64 string); mesh uses ports 4445/4455/4465 so the workspace conductor sits on 4485. Related: [[project_alpha_substrate_probe_rails]], [[project_local_mesh_binary_slot_and_restart]].

**2026-09-06 — fleet-parity conductor without a 45-min build.** The alpha fleet runs the ethosengine fork
REBASED ONTO stock holochain 0.7.0 (submodule `elohim-0.7` branch, gitlink 25dd2d0be; carries the
cross-relay preflight fix stock 0.7.0 lacks, which the two-relay alpha topology needs). Its image tag is
`harbor.ethosengine.com/ethosengine/elohim-edgenode:conductor-<hc12>` — **tx5 is gone from the tag on the 0.7
line** (CLAUDE.md's `conductor-<hc12>-<tx512>` is stale). Anonymous `skopeo` reads work; the fork binary is
`COPY … target/release/holochain /bin/holochain` in the 25th of 26 layers (lands as `usr/bin/holochain`),
while the BASE layer ships stock 0.6.0 `hc`+`holochain` at the same path — extract the fork layer ALONE or
the base overwrites it. Pair it with the stock 0.7 `hc` (`/projects/.claude-config/tools/hc-0.7/hc`).
Extracted pair lives at `/projects/.claude-config/tools/hc-fork-25dd2d0be144/bin` (holochain 0.7.0, sha256
283909f7…); use `MESH_FORK_BIN_DIRS=<that dir> just dev conductor alpha`. `hc-start.sh`'s "fork pair
required" check dates from the 2026-08-28 tx5 measurement and only tests that two binaries share a dir — the
honest predicate is "same conductor lineage as the fleet pin". Also: the working-tree gitlink for
`elohim/holochain-conductor` was found drifted to c9a6c4439 (a 0.6.3-lineage branch) — never stage a
gitlink you did not move on purpose; `git rev-parse HEAD:elohim/holochain-conductor` is the fleet pin.
