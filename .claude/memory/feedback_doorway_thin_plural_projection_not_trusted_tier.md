---
name: feedback-doorway-thin-plural-projection-not-trusted-tier
title: "Doorway = thin plural projection"
id: feedback-doorway-thin-plural-projection-not-trusted-tier
description: "Operator 2026-09-12: doorways hold only notarized projections under a plural contract — never a trust tier or warehouse."
metadata: 
  node_type: memory
  title: "Doorway = thin plural projection, not a trusted tier"
  type: feedback
  originSessionId: 2299131b-7b0d-45f5-b76e-362109f418b4
  modified: 2026-09-12T13:20:47.245Z
cites:
  - "memory-search-scale-three-seams-design | Memory, search and scale | sha256:c119695543a2854f | path: genesis/docs/superpowers/specs/2026-09-12-memory-search-scale-three-seams-design.md"
  - genesis/research/local-first-to-council-memory-search-seams-2026-09-12.md
---

Operator correction 2026-09-12, during the local-first-to-council memory research: I had written the doorway as "the trusted hosted tier that can read" and floated its MongoDB as a place for pool-scale indexes. Both framings are wrong.

What the doorway is: a projection, not its own thing. What it may hold is notarized by the holonic authorities that govern its relationship to the people it serves. It is federated and plural in contract, so data on the p2p dataplane cannot be captured by any single doorway. Its Mongo (account archive, custodial keys, signal bus, blob-metadata projection store) is all projection.

**Why:** the design goal is *thin* federation: cheap, easy, feasible for many doorway operators, to avoid the fediverse's recentralization failure where operators beg their audience for compute donations to pay Amazon. A warehouse-grade doorway is a fat doorway, and fat doorways are the capture route.

**How to apply:** in reach and search design, name the doorway as its contract (projection under notarized plural federation), not as a trust tier and not as a blind relay. Any pool-scale index or "big data" concern lives in arcs × custodians on the dataplane; a doorway participates only as a custodian under a compute grant, with the same shard format as a phone. Reject anything that makes a doorway heavier. Related: [[feedback_p2p_vs_federation_layer_vocabulary]], [[feedback_cleanup_toward_p2p_dataplane_trajectory]], [[feedback_human_loop_not_terminal_authority]] (same cage reflex in a different costume).
