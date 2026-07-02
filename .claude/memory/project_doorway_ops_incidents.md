---
name: project_doorway_ops_incidents
title: Doorway ops incidents (umbrella)
description: "Doorway ops incidents: A/B edges are islands (no coherence); kitsune2 bootstrap gap made conductors DHT islands; blocking getaddrinfo parks tokio workers."
metadata:
  type: project
---

# Doorway ops incidents (umbrella)

Folds the doorway operational-incident cluster. Members:

- [[project_doorway_ab_edge_islanding]] — two independent doorway edges over matthew/adam with no cross-edge coherence or divergence detection; e0352a7/8a2c65e glyphs were buildIds, not content CIDs.
- [[project_doorway_kitsune2_bootstrap_protocol]] — HC 0.6 conductors speak kitsune2 bootstrap (PUT /bootstrap/{space}/{agent}); doorway served only kitsune1 until 2026-06-12, leaving conductors DHT islands.
- [[project_doorway_wedge_unbounded_mongo_await]] — doorway-alpha SIGKILL crashloop root = blocking getaddrinfo in holochain_client connect parking tokio workers during DNS flaps; fixed via async DNS + watchdog.
