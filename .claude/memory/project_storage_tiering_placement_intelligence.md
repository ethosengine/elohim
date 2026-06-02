---
name: project_storage_tiering_placement_intelligence
description: "quilt blobs carry storage-tier awareness (cache→available→storage→archive) so RS-encoded shards land on the right hardware substrate via the compute-reporting surface; agent→hub derivation also unblocks device→hub aggregation scalability; modeled-in-genesis now, elohim-operator-tuned later"
metadata: 
  node_type: memory
  type: project
  originSessionId: 22de0299-7a43-43b5-a84a-e497e9397bbe
cites:
  - genesis/docs/superpowers/specs/2026-05-28-mutual-storage-replication-dwelling-hub-design.md
---

The "reed-solomon S3 shape" quilt (aka p2p-redis; see [[project_quilt_pantry_vocabulary]]) should carry **storage-tier awareness** — a placement axis spanning `cache → available → storage → archive`. The intent: a blob/shard is stored at the *right availability on the right hardware substrate*, so hot content sits on fast cache-tier peers and cold/archive content settles on slow bulk-storage peers.

Tiering decisions ride on top of the **compute-reporting surface** (the per-node probes + observations + peer archetype/capacity signals — see [[feedback_check_existing_compute_foundation]], [[project_compute_and_model_independent_diversity_surfaces]], [[project_storage_as_pod_operator_sets_virtual_limits]]). That surface exists precisely to enable a *diversity of peers* and *intelligent storage decisions* on top of the deterministic substrate floor ([[project_substrate_floor_elohim_ceiling]]).

Sequencing doctrine: the placement/tiering decisions are **modeled in genesis now** (deterministic, hand-tuned), and become **elohim-operator-tuned later** as the network matures — i.e. the substrate floor decides deterministically today; real per-node elohim agents add discernment/tuning once they can be added past the modeled decisions. Build the tier-awareness *hooks* into the substrate (wire hints, placement metadata) now so the maturity path is open.

**Why:** surfaced 2026-05-29 during the Epic-B prioritizer wire-format design (sprint/cross-pillar-cleanup). The replication prioritizer + inventory-gossip wire extension is the first place tier-awareness can be threaded (advertise + match on tier alongside recipient_hub_id/epr_kind/size).

**How to apply:** when extending the inventory-gossip wire / replication_prioritizer / shard placement, include a storage-tier dimension (don't hardcode a single availability class). The `peer_id→agent_cid→dwelling_hub_id` derivation needed for the prioritizer's `recipient_hub_id` hint is *also* the missing piece for **device→hub aggregation / DHT→projection scalability** — a recurring past concern (see [[project_node_metrics_vs_hub_aggregation_boundary]], [[project_hub_archetype_abstraction]]); build it as a reusable mapping, not a one-off.
