---
name: household-nodes-is-the-stable-floor
description: "Don't conflate degraded-6peer-soak/shem-offline with \"content work blocked\" — the M/J/J household is a live multi-peer mesh; prove deep there"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 5202e82d-a14d-4ad3-b13a-966c13b597d3
---

When classifying a2o reds as BLOCKED-BY-ENV, do NOT conflate "`alpha-cluster-6peer` degraded / `shem` offline" with "this work is blocked." `cluster-state.yaml` declares `household-nodes` (Matthew/Jessica/James) `available: true`, and a 3-node household is itself a **live multi-peer P2P mesh** — it satisfies the publish drain's `≥1 connected peer` gate, runs conductor→storage signals, and forms collectives. Verified 2026-06-08: `curl https://doorway-alpha.elohim.host/health` → `"p2p":{"peerCount":2}`, conductor 4/4, `discoveryComplete:true`.

**Why:** the operator's principle (2026-06-08) — *go deep on the most stable architecture (household-nodes); minimize the test surface that needs greater complication (shem / 6-peer soak / federation) to prove.* I mis-labeled household-provable reds (household-formation `collective_cid`, "Read own content" 404, stewardship affinity) as blocked and nearly added a false `@requires:p2p-publish` HELD — which would have **hidden real, foundation-hardening household bugs**. Holding household-provable work behind a remote cap is leaving value on the table.

**How to apply:** (1) Before calling a content/P2P red "blocked," check its `@requires:` tag — the cross-node minority (cross-human content *discovery*, e.g. "Susan discovers Matthew's content") is correctly `@requires:shem`; almost everything else (own-content read/write, provenance publish, collective formation, multi-peer gossip among M/J/J) is household-provable and should PASS there, not be held. (2) Probe `doorway-alpha /health` `p2p.peerCount` to confirm the mesh is live before blaming "no peers." (3) "Read own content" 404 is a **design-coupling** bug, not env: own-content reads must resolve at the local-write layer, never wait on a DHT-publish round-trip (that's the [[single-node-content-provenance-visibility]] / hub-optional-floor fix — see [[project_hub_optional_floor]], [[project_shem_is_p2p_live_canvas]]). CLAUDE.md already states "shem ≠ multi-node; the household is a 3-node cluster, household-testable" — this is the trap of forgetting it under a degraded-cluster headline.
