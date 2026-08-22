---
name: project_content_sync_plane
title: Content sync / replication plane (umbrella)
description: "Sync-plane state: inventory gossip is metadata-only, not bytes; Automerge plane lit; iroh dual-stack; ghost declared heads deadlocked batch-3; ingest drain solved twice; local seed never DHT-anchors."
metadata:
  node_type: memory
  type: project
---

# Content sync / replication plane (umbrella)

Folds the content sync / blob-replication dataplane cluster. Members:

- [[project_inventory_exchange_not_byte_replication]] — Inventory gossip is metadata-only ("Received content inventory count=N"); byte replication is separate — check per-peer blob counts before calling sync alive.
- [[project_automerge_content_sync_plane_lit]] — Automerge storage-sync plane LIT (producer + libp2p convergence proof); producer MUST write h_app_id="elohim"; back-fill default-ON w/ fleet-safety invariants.
- [[project_ghost_declaration_deadlock_batch3]] — The "~2000 unanchored rows" were anchored rows with phantom declared heads (dead incarnations); cure = local-get responder + author-over-ghost decay (ELOHIM_GHOST_DECLARATION_DECAY)
- [[project_closed_loop_ingest_drain_prior_art]] — Paced ingest drain solved TWICE; live kernel = drain_publish_queue + wait-for-drain; warm_stream is open-loop pacing — diagnose the hang before a 3rd scheme.
- [[project_local_stack_dht_anchor_gap]] — Local bulk seed never DHT-anchors → provenance gate 404s all reads by design; dev repair = p2p_published_at backfill; real fix = import anchor step
- [[project_iroh_dataplane_actual_state]] — Dual ENABLED in alpha manifests 2026-08-05 (deploy pending) w/ sovereign never-n0 iroh defaults; proof = "Dual: DualGossipPublisher wired into P2PNode" + irohNodeId, NOT the degraded-wrapper log.
- [[project_dataplane_next_lens_diversity_placement]] — Diversity-aware salvage placement (1a+1b landed) is INERT in prod — household_id NULL from identity-coherence gaps, not scope reads; degrades safely to XOR.
