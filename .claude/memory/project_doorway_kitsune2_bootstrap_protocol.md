---
index: false
name: doorway-kitsune2-bootstrap-protocol
title: Doorway kitsune2 bootstrap protocol (HC 0.6)
description: "HC 0.6 conductors speak kitsune2 bootstrap (PUT /bootstrap/{space}/{agent}); doorway served only kitsune1 until 2026-06-12, leaving conductors DHT islands."
metadata: 
  node_type: memory
  type: project
  originSessionId: da9eca9d-9d7f-4a87-83f4-1f4197e6beba
---

The doorway's bootstrap originally implemented only the kitsune1 protocol (`POST /bootstrap` + `X-Op: put|random|now`, MessagePack + Ed25519). Holochain 0.6's kitsune2 client speaks a different wire: `PUT /bootstrap/{spaceB64url}/{agentB64url}` (no-pad) with JSON `{"agentInfo":"<inner JSON string>","signature":"<b64>"}` (sig over the exact inner-string bytes), polled `GET /bootstrap/{space}` returning a JSON array of raw stored bodies. Those PUTs fell through the route registry to 404 → zero conductor peer discovery → no kitsune gossip → every DHT entry stayed on its authoring node (root cause of propagation.custody-convergence; also adam's "Subject doorway not found"). Client-side tell: `kitsune2_core core_bootstrap "Bootstrap overloaded, dropping put Full(..)"` spam (~1500/hr) as the put queue backs up on failing requests; server-side tell: doorway INFO `PUT /bootstrap/...` paired with DEBUG `No registry match and no SPA fallback`.

**How to apply:** kitsune2 handlers live in `doorway/doorway-service/src/bootstrap/k2.rs` (validation mirrors kitsune2_bootstrap_srv v0.3.0-dev.3: ±3min created_at skew, ≤30min expiry span, MICROsecond timestamps, path-vs-body match, tombstone deletes). Residual: the store is in-memory PER DOORWAY POD — multiple replicas shard the agent-info view; client re-put/poll cadence converges it, but if cross-peer discovery is flaky with >1 doorway replica, suspect this first. Conductor-plane (kitsune) networking is invisible to the mesh.adjacency gate (which tests the libp2p storage plane) — green mesh ≠ conductors can gossip.
