---
id: "backlog-collectives-arm-bootstrap-gap-no-stamped-cid-anywhere"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Collectives reconcile arm cannot bootstrap when NO pod holds a stamped cid — needs a DHT-enumeration discovery leg"
slug: "collectives-arm-bootstrap-gap-no-stamped-cid-anywhere"
written: "2026-07-28"
author: "heads-converge-truthful-resilience shift"
status: "open"
priority: "medium"
tags: [collectives, projection-reconcile, dataplane, resilience-card, qahal, brainstorm]
cites:
  - genesis/data/timeline/backlog/content-divergence-unhealable-without-canonical-heads.md
---

# Collectives arm bootstrap gap — inventory-only discovery is circular when the fleet starts empty

The collectives reconcile arm (landed 2026-07-28, commit 33225a8ae) discovers
gaps from PEER INVENTORY, and the inventory deliberately filters to
`collective_cid IS NOT NULL` (pre-coherence seed rows have no DHT identity to
heal from — reconciling them would replicate local-only garbage). Verified on
alpha 2026-07-28: `collectives_ids_discovered=0` from all 6 peers on every pod
— household-dowell exists only as a NULL-cid seeded SQL row on the A-side
backend, so the arm has nothing to carry and `stewardingCollectives` stays
0-vs-1 across doorways (ch10 red).

**The structural gap:** when NO pod has a stamped cid, inventory-only discovery
is circular — no inventory entry can ever exist to bootstrap the first stamp.
The content arm never had this problem because content rows acquire
`dht_anchor_hash` at authoring on every conductor.

**Design question (brainstorm-classed, do NOT grind loops on it):** should the
arm grow a DHT-enumeration discovery leg — own-conductor enumeration of
Collective entries (the identity_fill / peer_status_fanout precedent: periodic
own-conductor-DHT-pull, create-capable, fills-never-moves) — so a
DHT-committed collective reaches every projection even when zero pods hold a
stamped row? Prerequisite either way: the household formation itself must be
DHT-committed (ceiling: per-member household formation on alpha — RCA finding
3); with an empty DHT both discovery legs are correctly, truthfully empty.

Status: OPEN (design). The p2p-design-gate classification for any new leg is
already settled (A-projection repair, no new truth source).
