---
id: "backlog-runtime-shem-edgenode-container-exit-139-chronic"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The shem-node edgenode containers (susan, eve, gertrude) die with exit 139 (SIGSEGV) several times an hour under seed load — chronic across two storage builds, no Rust panic line, and every genesis run accelerates it"
slug: "runtime-shem-edgenode-container-exit-139-chronic"
written: "2026-08-28"
author: "shift 2026-08-28T03-25-shakeout-landing-perf-trust-hybrid (post-close)"
status: "open"
priority: "high"
jobs: [elohim-edge, elohim-genesis]
tags: [runtime, conductor, shem, sigsegv, restart-churn, measurement]
---

## Measured (Prometheus kube-state + cadvisor, 2026-08-27 → 2026-08-28T14:45Z)

- `kube_pod_container_status_last_terminated_exitcode{container="elohim-node"}` = **139** on susan, eve, gertrude and matthew (reason `Error`, not `OOMKilled`; working sets 13–53 % of limit).
- Restart counters (reset at each pod recreation: edge #1386 ~23:26Z 08-27, the 09:29Z cluster reboot, edge #1388 ~11:50Z 08-28): on **9a207277** 08-27 daytime eve 8→10 · gertrude 15→16 · susan 5→6; 08-27 night eve 1→4 · gertrude 0→4 · susan 1→6; on **6e4fa438** 12:20–14:40Z susan 1→4→10 · eve 1→4 · gertrude 2→4 · matthew 0→1. adam/james/jessica: 0 throughout. So the class predates the 2026-08-28 storage diff (two commits, no `unsafe`/FFI) and lives on the shem-node pods (3 CPU / 3 GiB, CPU-throttled 75–99 % for hours).
- Today's acceleration (susan 9 deaths in 2 h) coincides exactly with genesis #1512/#1513/#1514 (12:38–14:37Z) — three consecutive seed storms dispatched by docs-only pushes (dispatch over-trigger, fixed in the same commit as this entry). Each seed storm = restart churn = the E2E measure reds on doorway unavailability; genesis #1513 (no churn in front of it) measured 2 failures, #1514 (mid-churn) measured 6.
- No `panicked at` in any elohim-node log over the window; the pre-death lines on susan are the HcClient reconnect loop hitting `CellDisabled(imagodei)` on her own conductor plus `attach_app_interface AddrInUse` — a container that is restarting faster than its conductor re-enables its cells. Which process takes the SIGSEGV (storage vs the in-container conductor, jemalloc/kitsune2/tx5 native code) is NOT established — Loki carries no core-dump marker; the kubelet's container exit is the only signal.

## Why it matters

The shem trio is the multi-tenant canvas (`@requires:shem`); a peer that segfaults every 10–15 minutes never finishes a reconcile sweep, so the fleet's `converged=0` plateau has a mechanical floor here, and susan is the fleet's expensive sync edge (`sync-edge-susan-timeouts-per-edge-observability`).

## Next (ordered, first two bounded)

1. Runtime: capture the signal — `RUST_BACKTRACE`/a SIGSEGV handler that logs the faulting thread name (storage) and the conductor's own crash output to stdout so Loki carries it; or `kubectl describe`/`dmesg` on shem by the operator to see which PID faulted.
2. Pipeline: the genesis dispatch over-trigger is fixed (`genesis/build-manifest.json`: `genesis/data/**` → the consumed subdirs; timeline prose no longer dispatches a seed storm). Separately, gate genesis's E2E on doorway `caughtUp` sustained after its own doorway restart (see `ci-genesis-doorway-503-seed-phase-wedge`).
3. Substrate: raise the shem pods' CPU (3 → 4+) or stop co-scheduling three tenants on one 3-core node during seeds; conductor-saturation class per `project_conductor_arc_resources`.
