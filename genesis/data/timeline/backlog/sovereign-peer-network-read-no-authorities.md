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

## 2026-08-30 — T3 workspace peers cannot receive coordinator hot-swaps

A storage attached to an EXTERNAL conductor (`HOLOCHAIN_ADMIN_URL`, the T3 rung's shape) never runs
`happ_manager::ensure_happ_installed`/`sync_coordinators` — that path lives inside the embedded
ProcessManager boot loop only. So when a coordinator-only change ships (e.g. the head-delegation
batch), the fleet hot-swaps via its embedded storages but a workspace conductor keeps the old
coordinators; the only workspace path today is a fresh `hc sandbox generate` (which re-keys the
device agent). Cure candidate: an `--sync-coordinators-once` storage flag (or a small admin script)
that drives `update_coordinators` against an external conductor from `--happ-path`, preserving the
device key. Until then a T3 re-key after each coordinator wave is the documented cost.

## 2026-08-30 22:5xZ — confirmed by live test
`scripts/delegation-live-check.ts` (W grants to a fresh device key D, D declares the gate-reading
head) failed at the grant with `Zome content_store Fn grant_head_delegation doesn't exist` — the
workspace conductor (started 20:52Z from Jenkins lastSuccessfulBuild, pre-a0784c306) carries the OLD
coordinators. The fleet's 7 pods hot-swapped the new ones at 22:48Z; the workspace did not, exactly
because the external-conductor path skips `sync_coordinators`. Cure remains the `--sync-coordinators-once`
storage lever (reuse `happ_manager::sync_coordinators` against `HOLOCHAIN_ADMIN_URL`, key-preserving).

## 2026-08-31 — sharpest evidence: storage-plane caught-up, conductor-plane head NOT adopted
Device peer W2 authored the manifesto, re-notarized reach=commons, declared an EARNED cross-root
canonical head (valid + DHT-anchored on its own conductor). Measured after:
  - W2 storage/iroh plane: irohPeersKnown=7, replication caughtUp, pull caughtUp (CONNECTED).
  - matthew's fleet conductor (doorway-alpha resolve_content_head): canonical:false, old per-root
    head — it NEVER received W2's canonical-head link.
The STORAGE plane converges for a fresh joiner; the CONDUCTOR kitsune2 DHT-gossip plane does NOT
carry a fresh joiner's authored canonical-head link (StringAnchor link op) to the fleet, so no fleet
conductor's election sees it and the doorways keep projecting the old head. NOT a doorway-auth/admin-
key concern, and NOT (on today's scaffold) an authority concern — head election is a conductor-plane
property and a fresh joiner's ops aren't gossiped/fetched by the fleet (arc-null; no advertised
authority to pull from). Forcing it by declaring on a fleet conductor hits the 60s Network-resolve
timeout (gather_content_chain(Network) on a cold/saturated arc). Cure lives in the conductor/kitsune2
storage-arc + gossip path (make a fresh joiner an authority whose ops the fleet fetches) = the
dataplane-convergence habit. "Declare from the root author's conductor" masks this; the real bar is
any validly-elected head converging across the plane with every doorway projecting it, no key.
