---
id: "backlog-sovereign-peer-network-read-no-authorities"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A freshly joined iroh peer cannot read fleet content within minutes in EITHER arc mode — every live alpha agent-info advertises storageArc: null, so a network get has no authority to ask (measured on the fork pair, 2026-08-28)"
slug: "sovereign-peer-network-read-no-authorities"
written: "2026-08-28"
author: "shift 2026-08-28T17-40-m0-pawls-ratchet-lanes"
status: "open"
priority: "high"
jobs: [elohim-edge, elohim-holochain]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-p1-dht-authored-content-not-projected"
  - "backlog-iroh-lane-bootstrap-publish-dark"
  - "habit:dataplane-convergence"
tags: [dataplane, sovereign-peer, kitsune2, storage-arc, iroh, bootstrap, T3, ratchet-lane-P]
---

## Measured (T3 hybrid rung, workspace conductor = ethosengine fork holochain 0.6.3 on iroh, joined to alpha)

`features/deployment/sovereign-peer-join.feature` scenario 1 is GREEN on the fork pair (fingerprints match
doorway-alpha's five spaces, peer store 25–30 fleet entries, listed live by conductor-diagnostics, 4–5 iroh
connections incl. one direct). Scenario 5's workspace-peer read of `elohim-host-landing`
(`content_store::get_content_by_id`, HDK `GetStrategy::default()` = **Network**) answers `null` in 7–10 ms:

- **`target_arc_factor: 1`** (hc sandbox default): a full-arc node is its own authority for every hash, so
  the Network strategy answers locally; the node holds the fleet's data only as gossip fills its rings.
  Gossip ran at ~90 KB/min (3.29→3.74 MB in 5 min); the landing node was NOT held after 15 min of polling
  on a conductor up 65 min. Hours-scale for this corpus.
- **`target_arc_factor: 0`** (fork `hc … network --target-arc-factor 0 …`, a zero-arc reader whose gets
  must go to the network): still `null` in 7 ms after 3 min — the get had nobody to ask.
- **Why:** every live agent-info in alpha's bootstrap store (`GET /bootstrap/<space>`, 27 live agents,
  all five spaces) carries **`storageArc: null`** — including the fleet's own full-arc conductors. kitsune2
  encodes an EMPTY storage arc as null; an agent advertising no arc is an authority for nothing, so a
  remote peer's network get resolves to "no authorities" and falls back to its own (empty) cache. The
  fleet's own conductors read fine because each is full-arc *locally* (self-authority) — the same
  mechanism that makes a full-arc joiner read its empty cache.

## Why it matters (lane P rung P5/P6; habit dataplane-convergence)

The sovereign/hybrid dev rung can now JOIN (fork pair required — `hc-start.sh` refuses a stock tx5 join to
the iroh fleet), but it cannot READ the fleet and, by symmetry, nothing it authors can be fetched by
authority either (P1 gap, now over-determined by this). If the fleet's advertised arcs are genuinely empty,
it is also the mechanism behind cross-conductor `dht-fetch` misses and I3 root-author fallbacks: every
"remote" read on alpha is served by local full-arc caches, never by DHT authority.

## Next (ordered)

1. Confirm the arc on a fleet conductor directly (operator or Loki): `dump_network_metrics` on matthew —
   does its own agent-info show a non-empty arc, or does kitsune2's arc-growth never complete (it grows
   only once gossip verifies coverage — which the caughtUp=false plateau may be starving)? Compare with a
   tx5-era bootstrap record if any survives (`iroh-lane-bootstrap-publish-dark`).
2. If arcs are empty by fork/kitsune2 behaviour: the fix is in the conductor fork / kitsune2 config (arc
   growth, or `target_arc_factor` semantics on iroh), not in storage or a2o.
3. Story: `sovereign-peer-join.feature` sc5's "within 3 minutes" is a gossip/authority bound, not the
   storage sync-round bound its glossary borrowed; keep it RED until 1–2 resolve, then re-measure
   time-to-hold under arc 1 and network-read latency under arc 0 (`CONDUCTOR_ARC_FACTOR`).

## 2026-08-30 — re-measured on fork c9a6c4439 (workspace pair), fleet still on the base conductor

W (workspace, `target_arc_factor=1`) announced 03:41:14Z; doorway-alpha's conductor-diagnostics listed all
40 agent-infos (8 agents incl. W) with `storageArc: null` — W's OWN full-arc conductor advertises null too,
so the null is what a fresh kitsune2 arc looks like on the board, not a fleet-only defect. Whether it ever
grows is still unanswered — and the fleet's conductor was found NOT to be the pinned fork
(backlog/conductor-pin-ships-base-binary.md), so every prior "on the fork" fleet read of this item needs
re-doing once 9d2842a63 deploys. sovereign-peer-join scenario 1 PASSED on the pair.
