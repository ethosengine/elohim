---
id: "backlog-doorway-landing-humans-served-source"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Threshold landing 'Humans Served' — decide the canonical source/semantics (federation social aggregate vs doorway hosted-count) and wire it; currently honest-null"
slug: "doorway-landing-humans-served-source-2026-06-23"
written: "2026-06-23"
author: "oAuth+federation shakeout — counts contract fix"
status: "open"
priority: "low"
tags: [doorway, threshold-landing, metrics, humans-served, semantics, architect-decision]
relatedNodeIds:
  - backlog-wan-nat-federation-dataplane-discovery-gap
cites:
  - doorway/doorway-service/src/routes/status.rs
  - doorway/doorway-service/src/routes/admin.rs
  - doorway/doorway-app/src/app/components/landing/doorway-landing.component.ts
---

# Threshold landing "Humans Served" — source decision

## What landed (2026-06-23)

The threshold landing's `/status` fetch was broken (SPA fetched the **HTML** status page,
expecting JSON → parse failed → both stat cards showed `—` unconditionally). Fixed by
repointing the SPA at `/status.json` and adding the camelCase contract fields to
`StatusResponse`. **`contentAvailable`** is now wired to a real, always-available measure —
the doorway's projected substrate-entry count (`projection_documents`). **`humansServed`** is
deliberately left `None` (renders honest `—`, never a misleading `0`) because its source +
semantics is an unresolved design question.

## The decision needed (architect)

What should "Humans Served" count on a doorway's threshold landing?

- **Option A — federation social aggregate.** `admin.rs` already computes
  `total_humans_served` by summing self-reported `social_metrics.humans_served` across
  `all_nodes` (only `nodes_with_metrics` report). Federation-wide, but it's an
  orchestrator/heartbeat aggregate (not a substrate read), lives on the admin path (not in
  `build_status_data`), and is `0` on alpha until nodes self-report.
- **Option B — doorway hosted-human count.** The count of humans this doorway hosts
  (Axis 2 identity hosting — active `UserDoc`s). A doorway-local Operational measure, always
  available, and arguably what "served *by this doorway*" means. Cheap (`count_documents`).

A/B are different numbers with different meaning; the label "Humans Served" must match
whichever is chosen. Once decided, wire it into `build_status_data` and set
`humans_served: Some(..)` (keep the `Option` so genuinely-unmeasured stays `—`, not `0`).

## Note

This is decoupled from the substrate-read framing the old caption implied — neither figure is
a live DHT query. The corrected caption now describes each honestly (`contentAvailable` =
projection of substrate content; `humansServed` = federation aggregate). See
[[backlog-wan-nat-federation-dataplane-discovery-gap]].
