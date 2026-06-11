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

Residual (deliberate, not implicated in the #1122 evidence): the registry's
boot-time connect_role stays bounded for the infrastructure role (heartbeat,
peer-status subscribers read its Option synchronously at boot) — converting
it needs interior-mutable registry state; follow-up when that path shows up
in evidence.

shift_objective: |
  Verify on the next deploy that a rolling restart converges: bridge
  connects late, reconcile controller starts, conductor_missing drains to
  0 on non-genesis pods, junction stamps land.

## VERIFIED LIVE + DEEPER ROOT CAUSE (genesis #1123 window)

Registry roles confirmed healthy post-fix: imagodei connects at attempt
4-5 through the CellDisabled window, lamad at attempt 1, on all three
pods. BUT "Reconcile controller disabled" still appeared — and the reason
is a SECOND, older defect the retry noise had masked since inception: the
controller passed IMAGODEI_APP_ID (default literal "imagodei", set by NO
manifest) as the INSTALLED APP ID, while the conductor's installed app is
the elohim happ with imagodei as a ROLE. "app 'imagodei' not found" was
an app-id mismatch, not timing — the controller has never connected on
alpha, and the forever-retry would have retried the wrong id forever.
Fixed (this wave): the connect now mirrors the registry's working pair
(app_id = args.app_id, role = "imagodei"). Junction stamps +
MembershipProjected projection get their first real chance next deploy.
Also noted: the registry's `infrastructure` role fails all 5 attempts on
matthew/jessica (PeerStatus heartbeat disabled) — role-name vs happ
manifest question, same family, separate verification next run.
