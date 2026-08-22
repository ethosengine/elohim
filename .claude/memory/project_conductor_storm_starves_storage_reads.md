---
index: false
name: conductor-storm-starves-storage-reads
title: Conductor CPU storm starves storage reads — triage order
description: All-A-side saga reds + catching-up 503 + caughtUp flap = check CFS throttle/breaker BEFORE the identity plane; conductor spawns nice-10 since 3146ebdc5
metadata: 
  node_type: memory
  title: Conductor CPU storm starves storage reads — triage order
  type: project
  originSessionId: cb148cf7-2cdc-4c91-a807-6ea4d81cdbc9
  modified: 2026-08-17T21:12:09.638Z
---

Triage order for "every A-side dataplane probe red at once" (ch02/03/05/07 + identity-coherence, edge #1360 2026-08-17): it was NOT data — rows were correct. Chain: kitsune2 single-op fetch storm (relay peer catch-up) → elohim-node container 100% CFS-throttled at its CPU limit for hours → storage HTTP (same cgroup) never answers → doorway breaker OPEN (`doorway_upstream_breaker_open_total`, `backpressure_honored` stays 0) → all /db reads shed 503 catching-up (retryAfter 30 = breaker cooldown; storage's own admission shed is retryAfter 2).

Check first: `container_cpu_cfs_throttled_periods/periods` ratio on the peer pod, breaker counters on its doorway, then rows.

**Why:** the identity-cluster framing burned a day; availability sheds impersonate data loss whenever probes join through the shed surface.

**How to apply:** conductor child spawns nice-10 by default (`ELOHIM_CONDUCTOR_NICE`, elohim-storage process_manager.rs, 3146ebdc5) — verify the conductor pid's nice on the fleet before re-diagnosing. Residual fork levers (unfixed): kitsune2 single-op fetch batching; kitsune2/iroh per-request debug-log volume (~320 lines/s). Related: [[project_doorway_ops_incidents]], [[project_alpha_substrate_probe_rails]].
