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
