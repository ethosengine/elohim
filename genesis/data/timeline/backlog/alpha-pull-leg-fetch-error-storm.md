---
id: "backlog-alpha-pull-leg-fetch-error-storm"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "alpha's acquisition pull leg has fetched nothing in 24 h — 3.2k fetch_error/24 h, no fetched series, on a build that carries the fetched outcome"
slug: "alpha-pull-leg-fetch-error-storm"
written: "2026-08-29"
author: "item-1 fleet validation (read-only telemetry)"
status: "open"
priority: "high"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "habit:dataplane-convergence"
  - "backlog-pull-leg-drains-before-iroh-book-warms"
tags: [dataplane, acquisition, fleet, alpha, fetch-error, ratchet-lane-P]
---

## Measured (Prometheus, elohim-alpha, 2026-08-29 ~16:45 UTC; Loki was 502 all afternoon)

- `sum by (outcome)(elohim_acquisition_outcomes_total)`: **fetch_error 907, transport_failure 5** — no `fetched`,
  `store_error`, `no_db_conn` series at all. `increase(...[24h])`: fetch_error **3223**, transport_failure 92.
- The running build is edge #1389 (`71f310ce6`, 2026-08-29 04:10 UTC), which contains `b30512348` (2026-08-28),
  the commit that added `inc_acquisition_outcome("fetched")` — so absence is behaviour, not a build artefact.
- `elohim_acquisition_active_pins`: matthew 2, every other pod 0. `elohim_acquisition_dispatch_total` lifetime:
  iroh 792 / libp2p 120. `elohim_iroh_peers_known` = 6 on all seven pods (books warm; manifests accepted 16.7k).
- Household mesh on the same code: fetched 3569 / fetch_error 2 in one recovery — the leg works when the peer
  it asks holds the record.

## Reading

Two pins, ~900 dispatches, every one an error: the pinned head_refs are being asked of peers that answer with
something other than the record (`iroh acquisition: unexpected response` / `request failed` are the
`fetch_error` sites — `p2p/mod.rs` ~10531–10581). Either the pinned ids exist on no peer (a pin naming a
provider-owned row the fleet never held), or the fleet's iroh content-fetch responder rejects the request shape.
Bounded by the retry budget, so it is a slow constant burn, not a storm in bytes — but `pull.caughtUp` can never
be honest on matthew while it lasts.

## Probe

- Next deploy: `elohim_transport_route_total{reason}` and `elohim_acquisition_first_drain_total{outcome}` on
  matthew, plus the pin list (`/db/acquisition/pins` is not proxied by the doorway — read it on-pod or add the
  route to the doorway registry).
- Loki, when it answers: `{namespace="elohim-alpha"} |= "iroh acquisition"` — the `response`/`error` field names
  the class in one line.
