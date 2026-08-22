---
name: project_conductor_arc_resources
title: Conductor arc, memory & CPU incidents (umbrella)
description: "Conductor resource incidents: sys-validation spin pegs CPU; a CPU storm starves storage reads (check throttle before identity); glibc leak cured by jemalloc; RAM ∝ corpus at full arc."
metadata:
  node_type: memory
  type: project
---

# Conductor arc, memory & CPU incidents (umbrella)

Folds the conductor resource/arc incident cluster. Members:

- [[project_alpha_conductor_spin_root_cause]] — 2026-08-21 — every alpha storage pod pegged at CPU quota for 48h+ = the conductor's sys-validation spinning on 353–2849 unfetchable dependencies; the pool saturation (Util 4037%) came from an UNBOUNDED join_all local re-lookup, not the network fetch. Cure 1 lives on fork branch fix/sys-validation-unfetchable-deps-backoff (pointer not bumped).
- [[project_conductor_storm_starves_storage_reads]] — All-A-side saga reds + catching-up 503 + caughtUp flap = check CFS throttle/breaker BEFORE the identity plane; conductor spawns nice-10 since 3146ebdc5
- [[project_storage_metrics_surface_and_leak_verdict]] — Conductor OOM = native glibc-malloc leak in holochain child (go-pion exonerated); CURED 2026-06-19 by jemalloc swap. /metrics + smaps localizer live on alpha.
- [[project_per_node_memory_is_conductor_authority_arc]] — Per-node 2→4GB+ climb (james OOM) = conductor full-arc DHT working set; target_arc_factor defaults to 1 so RAM ∝ corpus; arc-factor<1 is the scale lever.
- [[project_full_arc_authority_disables_network_get]] — On a full-arc fleet every zome get/get_links is local-only — a link miss means gossip failed, not that the data is absent.
- [[project_conductor_signal_msgpack_decode_class]] — holo_hash in conductor msgpack signals = raw bytes; Value pre-pass or String mirrors silently drop the signal — decode typed HoloHashB64 (closed 2026-06-13).
