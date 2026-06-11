---
id: "backlog-storage-hcclient-boot-giveup"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Storage gives up the conductor bridge after 5 boot retries (CellDisabled window) — reconcile controller dead for pod lifetime"
slug: "storage-hcclient-boot-giveup"
written: "2026-06-11"
author: "agentic-developer (EPR durability shift, live-seeding Grafana read)"
status: "wip"
priority: "high"
ci_status: pending-verification
jobs: [elohim-edge, elohim-genesis]
tags: [substrate, storage, conductor-bridge, reconcile, boot-ordering, one-shot-init]
cites:
  - elohim/elohim-storage/src/main.rs
---

# HcClient boot giveup — one-shot-init disease, conductor-bridge instance

Fleet-wide at every rolling restart (quoted, 15:00Z window): "HcClient
connect failed — retrying (cells may still be CellDisabled)" attempt=1..5,
then "Reconcile controller disabled: imagodei conductor connection failed
(storage still serves blobs/HTTP without reconciliation)". A conductor
that takes minutes to enable cells after a restart bricks the bridge until
the next pod delete. Downstream: MembershipProjected junction stamps never
project, and projection-reconcile heals report conductor_missing on every
discovered id (the heal leg reads through this bridge family) — the
current front of the convergence chain (`conductor_missing:34, healed:0`
on all 13 non-genesis pods while discovery returns 34 ids). Fix (this
shift, pending CI): persistent reconnect with capped backoff; late connect
completes the same wiring as boot connect. Same pattern as the libp2p
persistent-peering fix.

shift_objective: |
  Verify on the next deploy that a rolling restart converges: bridge
  connects late, reconcile controller starts, conductor_missing drains to
  0 on non-genesis pods, junction stamps land.
