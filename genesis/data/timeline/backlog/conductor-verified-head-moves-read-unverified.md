---
id: "backlog-conductor-verified-head-moves-read-unverified"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A head move the node's own conductor verified still reads unverified for about thirty seconds"
slug: "conductor-verified-head-moves-read-unverified"
written: "2026-10-05"
author: "claude-opus-5-5 (shem, shift event-driven-head-delivery), at the operator's direction"
status: "open"
priority: "low"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-hosted-authored-content-has-no-storage-row"
  - "habit:dataplane-convergence"
tags: [head-plane, anchor-state, trustful-self, unit-agent]
---

Found 2026-10-05 on the household while proving event-driven head delivery (`cfaf868ed`).

## The gap

`dht_anchor_state = live` means this node's own conductor resolved the row's anchor. Since
`cfaf868ed` a stamp that moves `dht_anchor_hash` without that witness clears the verdict, so a row
never claims `live` for a head nobody here confirmed. Two stamps carry the witness
(`content_diesel::stamp_own_conductor_canonical_head`): `head_adoption::adopt_local` and
`projection_reconcile::project_authenticated_content_head`. Every other mover goes through the
plain stamp: `adopt_peer`'s declare, election-obey, `pointer_audit`, `courier_obey`, and the
`content_service` declare. Some of those did verify through the own conductor before stamping
(reported by the implementer, not re-read here).

Measured: an update to a row a peer already holds lands its exact head in 0.5 s and reads `live`
27–29 s later, when the trigger ladder or the anchor-verify pass confirms it. A new root reads
`live` with its head (1.2 s) on one peer and 45 s later on the other.

## What is owed

Read each plain-stamp mover and decide, per caller, whether the head it stamps was resolved by
this node's own conductor as canonical for that exact action. Promote those that were to the
witness stamp; leave the rest to the verify pass. A caller promoted wrongly launders a peer's
claim into `live`, so each promotion needs a test of its own.

## Probe

`genesis/local-dev/event-driven-head-delivery/probe.mts` on the household: the update event's
`live` seconds equal its `anchorMatch` seconds on both peers.
