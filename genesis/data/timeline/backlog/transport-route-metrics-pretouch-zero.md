---
id: "backlog-transport-route-metrics-pretouch-zero"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "elohim_transport_route_total / _path_rtt_ms / acquisition_dispatch_total are invisible on the fleet until the first decision — pre-touch the closed reason vocabulary at zero (the ACQUISITION_RECONCILE_OUTCOMES pattern) so a converged fleet reads 'no routing happened' instead of 'metric absent'"
slug: "transport-route-metrics-pretouch-zero"
written: "2026-08-29"
author: "m4-fleet-confirm shift"
status: "wip"
priority: "low"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
tags: [metrics, observability, transport-selection, C8]
---

Measured 2026-08-29 06:4xZ after edge #1389 deployed 1.0.0-dev-71f310ce to 7/7 alpha pods: Prometheus lists the
iroh sync/gossip families on every pod but none of the transport self-awareness families — `IntCounterVec`s only
materialise a series on first `with_label_values`, and a converged fleet routes nothing. Absence and "not yet
decided" are indistinguishable, which is exactly the C4 trap at the metrics layer. Cure: in `metrics.rs`
registration, `inc_by(0)` every (transport × op_class × reason) of the closed vocabulary (the pattern already used for
`AcquisitionReconcileOutcome::ALL`), and pre-touch `elohim_acquisition_dispatch_total{transport}` for both planes.

## 2026-08-29 cure landed (local evidence; fleet read pending)

`ROUTE_REASONS` (transport_paths.rs) is the closed vocabulary; `register_all` pre-touches
`elohim_transport_route_total` over {libp2p,iroh} × {small,bulk} × every reason but `no_plane` (+ `none`×`no_plane`)
and `elohim_acquisition_dispatch_total` for both planes. Pinned by `every_emitted_reason_is_in_the_closed_vocabulary`
(walks the selector's full state × pick matrix) and `transport_route_vocabulary_is_pretouched_at_boot` (scrape text).
Next fleet deploy makes "nothing routed" read as `0`, not absence.
