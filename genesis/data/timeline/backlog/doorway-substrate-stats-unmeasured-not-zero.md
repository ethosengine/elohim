---
id: "backlog-doorway-substrate-stats-unmeasured-not-zero"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Doorway /status should MEASURE + expose substrate stats (humansServed, contentAvailable) as null-when-unmeasured — the service half of the layered-honesty degraded state"
slug: "doorway-substrate-stats-unmeasured-not-zero"
written: "2026-06-22"
author: "shift 2026-06-22T1550-dev-buildall-shakeout-doorway-503 (operator: enrich doorway threshold, layered visibility)"
status: "backlog"
priority: "medium"
tags: [doorway, ux-honesty, unmeasured-vs-zero, status, substrate, dataplane, layered-visibility]
relatedNodeIds:
  - backlog-resilience-unmeasured-vs-zero-honest-denominators
cites:
  - genesis/data/timeline/backlog/resilience-unmeasured-vs-zero-honest-denominators.md
  - doorway/doorway-service/src/routes/status.rs
  - doorway/doorway-service/src/server/http.rs
  - doorway/doorway-app/src/app/components/landing/doorway-landing.component.ts
---

# Doorway substrate stats: measure-or-unknown, never zero (the service half)

## What landed (the UI half — done 2026-06-22)
The degraded experience is now **layered-honest** on the read side:
- **Root `/` (public)** — when the EprRouter is empty (conductor unreachable), the doorway
  serves a layered 503 page (`root_unavailable_html`) that names Layer 1 (this doorway /
  web2 projection — federation peers + projection cache, reported live) and says Layer 2
  (substrate dataplane — DHT/libp2p/iroh via the conductor) is **not visible**, so
  humans/content/peers are **unknown, not zero**. (Replaces the old `302 → /threshold`
  operator-dashboard leak.)
- **Threshold landing** (`doorway-landing.component.ts`) — substrate stats render `—`
  (never `0`) when unmeasured, grouped by layer, with a caption.

## The gap (the service half — this item)
The doorway's `/status` JSON does **not actually measure** `humansServed` /
`contentAvailable` (the landing's `StatusResponse` declares them but the handler never
populates them → they arrive `undefined` → the UI fix renders `—`). To show *real*
numbers when the substrate IS reachable (and `null` only when it genuinely isn't), the
service must:

1. **Measure** humansServed / contentAvailable from the substrate (via the conductor /
   storage projection) — the doorway has the layered truth already in
   `build_status_data` (`conductor.connected`, `storage.reachable`, `cache.entries` —
   `routes/status.rs`); extend it to aggregate the dataplane figures.
2. **Expose** them in the `/status` JSON as `Option` (serialize `null`) — present a real
   count when measurable, `null` when the conductor/dataplane is unreachable. Never `0`
   for "couldn't measure."
3. The landing already renders `null` honestly (`—` + layered caption) — no further UI
   change needed once the service returns real/`null` values.

## Why it matters
This is the service-side completion of the same principle as
`backlog-resilience-unmeasured-vs-zero-honest-denominators`: **unmeasured ≠ zero.** The UI
no longer lies, but the doorway still can't *show the real numbers* when the substrate is
up — it has no substrate-stats aggregation for the landing. Pairs with the conductor-leak
work (the recurring cause of the doorway losing substrate visibility).
