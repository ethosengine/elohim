---
id: "backlog-epr-blob-replication-direction"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "EPR / blob byte-replication across alpha peers — pick fan-out-seed vs substrate P2P-replication"
slug: "epr-blob-replication-direction"
written: "2026-06-02"
author: "cartographer"
status: "proposed"
priority: "high"
area: "storage/SSR"
recurrence: 4
source_shifts:
  - "2026-04-29"
  - "2026-05-06"
  - "2026-05-23"
  - "2026-05-26"
domain: "operator"
relatedNodeIds:
  - "memory:project_inventory_exchange_not_byte_replication"
  - "memory:project_doorway_single_target_no_fanout"
  - "memory:project_p2p_is_hosting"
  - "memory:feedback_head_vs_get_blob_asymmetry"
tags: [storage, ssr, epr, blob, replication, p2p, operator-domain, recurring, direction-decision]
shift_objective: |
  EPR atoms and blob bytes don't replicate across the alpha peers: project-epr commitments
  and SPA blobHash content authored on one peer are invisible cross-peer, because gossip
  exchanges inventory/metadata only — not the bytes (project_inventory_exchange_not_byte_replication).
  This is the single most-recurring open thread in the shift corpus (2026-04-29, 05-06, 05-23,
  05-26) and it blocks cross-doorway EPR delivery and cross-peer SSR.
  It needs a DIRECTION DECISION, not just a fix: either (a) fan-out seed (the seeder/genesis
  writes content to every peer — simple, but violates doorway single-target and doesn't make
  the substrate self-healing), or (b) substrate P2P byte-replication (peers pull missing
  bytes on demand / on inventory-diff — the project_p2p_is_hosting direction, harder but
  the real shape). This is operator+architecture domain because it sets substrate direction;
  surface the trade-off and the conductor-agent-info-gossip "step zero" dependency. Done when
  the operator has chosen a replication direction and the chosen path has a tracked plan;
  cross-peer blob fetch should be probed with GET, not HEAD (feedback_head_vs_get_blob_asymmetry).
---

# EPR / blob byte-replication across alpha peers — direction decision

## Why this matters

This is the highest-recurrence open thread in the shift corpus (four shifts) and it sits at
a real architectural fork, which is why it needs an operator/architecture *decision* before
any shift implements it. Replicating the wrong way (naive fan-out) would entrench a pattern
the substrate vision rejects (`project_doorway_single_target_no_fanout`,
`project_p2p_is_hosting`).

## The failure shape

- An EPR atom or blob authored on peer A is invisible on peer B.
- Gossip propagates inventory/metadata only — the bytes never move
  (`project_inventory_exchange_not_byte_replication`). Check the filesystem count, not the
  gossip log, to confirm mobility.
- Cross-doorway EPR delivery and cross-peer SSR both depend on this; conductor agent-info
  gossip ("step zero") propagates DHT entries cross-mesh but does NOT move bytes or project
  them remotely.

## The fork (decide before building)

1. **Fan-out seed** — seeder/genesis writes to every peer. Simple; but violates single-target
   dispatch and isn't self-healing (a peer that joins later stays empty).
2. **Substrate P2P byte-replication** — peers pull missing bytes on inventory-diff / on
   demand. Harder; but it *is* the hosting layer (`project_p2p_is_hosting`) and self-heals.

## Acceptance

Operator has chosen a replication direction; the chosen path has a tracked plan. (Probe
cross-peer blob fetch with GET, not HEAD — `feedback_head_vs_get_blob_asymmetry`.)
