---
name: Per-node metrics vs household-hub aggregation boundary
description: system_metrics-style modules are single-node only; cross-node aggregations (sum of CPU counts, total compute, household-wide capacity) belong to the household-hub surface, designed later
type: project
originSessionId: 72a4534a-dd50-4984-be17-9d287ef54e6b
---
Per-node compute observations (filesystem capacity for *this* pod, RAM/CPU/RSS for *this* process) live in modules like `services/system_metrics.rs`. They surface single-node primitives and stop there.

Cross-cutting aggregations across multiple devices owned by one human (sum of CPU cores across desktop+node+steward, total committed bandwidth, household-wide free storage) are **emergent properties of the household-hub surface**. The hub abstraction is on the back-burner per `project_hub_archetype_abstraction` and `project_elohim_hub_elevation` — designed once the per-node primitives are mastered.

**Why:** Aggregation logic mixed into node-level modules forces them to know about peer relationships, household membership, and federation timing — concerns that belong in the hub. Keeping the boundary clean now means hub design has room to choose its own composition pattern (whether hub aggregates at request time, caches, or pushes via signal) without unwinding entanglements.

**How to apply:**
- When adding a new metric to `services/system_metrics.rs` or any peer-local probe, ask: is this an observation about *this* node, or a sum/avg/count across nodes? If aggregation, stop and surface as a hub-design concern instead.
- The cluster_view's federation aggregator (`services/cluster_view.rs`) is the *current* placeholder for cross-node rollups. When household-hub lands, that aggregation logic likely moves out.
- Don't preemptively expose aggregation interfaces at the node level. A function returning `Vec<DeviceSummary>` from one node = wrong shape; that's the federator's job.
