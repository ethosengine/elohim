---
name: feedback-cleanup-toward-p2p-dataplane-trajectory
title: Per-host scaffold — clean up toward P2P dataplane
description: "Per-host/per-row blob modeling is MVP scaffold toward the synced P2P dataplane — clean up drift as you go, never extend the scaffold."
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 32ed30bb-9c4a-4a71-9026-524a934f5f9e
---

The two-host model (alpha.elohim.host + elohim.host as separate storage backends, each PATCHed per-host) and per-row deploy artifacts (e.g. the separate `elohim-host-landing-ssr` content row holding the server bundle's blob pointer) are **MVP conveniences on the way to a fully-synchronized P2P dataplane** — content-addressed byte replication; substrate-replication trajectory home `genesis/docs/content/elohim-protocol/architecture/2026-05-11-tiered-quilt-stewardship-design.md` + the 2026-06-14 dataplane plan arc. Blobs do NOT auto-replicate yet (gossip carries inventory/who-has-what, not bytes — `Jenkinsfile:328-331`), so the slug→blobHash pointer is a per-host mutable write. That is a known temporary scaffold the user is actively building past, NOT the target architecture.

**Why:** The user is doing foundational work toward the synchronized dataplane and asked (2026-06-26) that I "clean up as you go so we stop making these fundamental errors of the trajectory." AI agents (including me, this session) default to **mirroring/extending the scaffold** — the relational/per-row reflex: a second content row per facet, a per-host PATCH, a CID-as-mutable-column. That drifts development AWAY from the P2P-native target and re-introduces the very divergence class (alpha vs elohim.host serving different bundles) the dataplane is meant to erase. These are "fundamental errors of trajectory," not local bugs.

**How to apply:** When work touches a data model or deploy mechanism involving per-host blob/pointer state, or a "seed a second row for a facet" pattern: recognize the scaffold and model toward the **P2P-native target** — ONE content-addressed EPR node carrying its full nature (browser ref + server ref + SSR-capable + theme as fields, not sibling rows); content is the "what", the serving peer's advertised capability is the "how" (the runtime already has `RenderCapabilityProfile` + a CSR fallback — stop fighting it with content rows). **Collapse redundant per-host modeling, and NAME any remaining per-host bits as temporary scaffold citing the dataplane spec** so the next reader/agent doesn't re-extend it. Cleanup-toward-the-trajectory is expected of every change, not optional. Upstream guard: the `p2p-design-gate` skill; related: [[feedback_p2p_vs_federation_layer_vocabulary]], [[project_inventory_exchange_not_byte_replication]], [[project_principle_p1_reconciliation_controller]].
