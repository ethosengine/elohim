---
name: project-edge-happ-fetch-race
title: Edge happ fetch races DNA publish in same wave
description: Edge bakes elohim-happ:dev-latest fetched mid-build; same-wave dispatch with the DNA pipeline ships the PREVIOUS bundle — dependsOn is not wave-ordered.
metadata: 
  node_type: memory
  title: Edge happ fetch races DNA publish in same wave
  type: project
  originSessionId: 3df467aa-5567-4c95-aaae-afaf9021f230
  modified: 2026-07-25T00:47:47.040Z
---

The edge pipeline (`elohim/holochain/Jenkinsfile`) does not build DNA — it `oras pull`s `harbor.ethosengine.com/ethosengine/elohim-happ:dev-latest`, which the DNA pipeline (elohim-holochain) publishes at the END of its run. `elohim/holochain/build-manifest.json` declares `dependsOn: ["elohim-holochain"]`, but on 2026-07-24 orchestrator #1521 dispatched both in the same wave (identical start timestamps): edge #1226 fetched dev-latest ~50 min before DNA #1370 published the new one, so the image shipped the previous happ. Symptom signature: pods log `No coordinator-zome drift` (bundle == installed == old) while consumers WARN `Attempted to call a zome function that doesn't exist` — a green deploy that silently misses a zome change.

**Why:** an artifact dependency (fetch-at-build-time) needs wave ordering, not just trigger propagation; the declared dependsOn is evidently not enforced as level ordering by graph-walker/build-graph.

**How to apply:** after any push touching both `elohim/holochain/dna/**` and edge-consumed code, check whether the DNA build *published* before edge *fetched* (edge log: "Fetching hApp from Harbor" vs DNA log: "Pushed [registry] … elohim-happ:dev-latest"). If raced, one `[build:edge]` empty-commit retrigger cures it. Durable fix candidate: enforce dependsOn as wave ordering in `genesis/orchestrator/build-graph.groovy`. Same shape as [[project-ssr-first-deploy-seed-then-restart]] and the coordinator-hot-swap path in [[project_dna_hash_blind_to_coordinator_zomes]].
