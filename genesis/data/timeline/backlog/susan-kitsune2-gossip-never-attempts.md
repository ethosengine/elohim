---
id: "backlog-susan-kitsune2-gossip-never-attempts"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "susan (shem) emits zero kitsune2 gossip attempts — a different failure mode from the fleet's relay-seam failures"
slug: "susan-kitsune2-gossip-never-attempts"
written: "2026-08-08"
author: "integrator-session"
status: "backlog"
priority: "high"
tags: [alpha-cluster, kitsune2, gossip, susan, shem, dht, relay-seam]

relatedNodeIds:
  - "backlog-edge-deploy-ready-gate-liveness-only"
---

# susan never attempts kitsune2 gossip (unique in the fleet)

Operator cluster pass, 2026-08-08, identical 30-min windows on image
`1.0.0-dev-ecc840eb`: shem siblings attempt gossip constantly and fail 100%
on the relay seam (`Failed to initiate gossip: "Connection attempted before
home relay URL is known"` — adam 57/57, eve 531/531, gertrude 538/538
failed), while **susan attempts zero times**. Never-attempts is a distinct
failure mode from attempt-and-fail — upstream of the relay seam. Her
elohim-storage layer is healthy (DualGossipPublisher inventory snapshots
once/min, sequence advancing). Lead (unverified): susan logs
`iroh_gossip::net: received command for unknown topic`. From conductor logs
susan carries 2 DNA hashes, matching all four shem peers (weak proxy).

Facts that bound the search: susan is a deliberate, documented active peer
(2026-07-02 cast directive; Seattle leg of the tri-region backup chain) —
not deploy drift. She is the ONLY `device-recycled-laptop` archetype on
shem (gertrude/eve are home-nucs), so archetype-derived render values are
the first susan-vs-siblings diff to check.

## Open legs

1. Repo-side: full effective-config diff susan vs eve/gertrude through the
   template render path; gossip-topic derivation behind the unknown-topic
   lead; git history of her rows (2026-07-16 archetype realignment).
   (Investigation dispatched 2026-08-08.)
2. Cluster-side (operator): whether susan's conductor installed/activated
   the cell — needs conductor admin API, blocked on shem kubelet serving
   cert (valid for 192.168.86.36, node joins as 10.99.0.2; `microk8s
   refresh-certs` on the shem host unblocks exec).
3. Note for measurement hygiene: a peer emitting zero DHT gossip is
   unlikely to contaminate F2 quiesce/AIMD baselines — but any fleet-wide
   convergence claim ("fleet quiesced") that counts susan is overstating.
