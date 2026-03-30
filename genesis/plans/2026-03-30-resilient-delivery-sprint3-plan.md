# Resilient HTML5 App Delivery — Sprint 3: P2P Mesh Delivery

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.
>
> **REVISIT BEFORE EXECUTING:** This plan was written during Sprint 1 planning. Review and update based on Sprint 1+2 outcomes before executing.

**Goal:** Peers serve peers. Client resolves multiple peers via EPR knownLocations, scores by capability/proximity, and falls back gracefully through the peer list.

**Architecture:** Extend EPR protocol with QueryDelivery message. Client-side ContentResolverService gains multi-peer scoring. Service Worker gains peer fallback chain. Tauri nodes serve each other on LAN via mDNS.

**Tech Stack:** Rust (elohim-storage P2P, steward/node), TypeScript (Angular ContentResolver, SW), libp2p (EPR protocol extension)

**Design:** `genesis/plans/2026-03-30-resilient-html5-app-delivery-design.md`

**A2O Scenarios:**
- `genesis/a2o/features/delivery/peer-mesh.feature` — 10 scenarios (LAN peer discovery, multi-peer resolution, fallback chain, QueryDelivery protocol)
- `genesis/a2o/features/elohim/network-health-posture.feature` — 19 scenarios (aggregate posture, attestation-gated introspection, elohim agent reasoning)

---

### Task 1: EPR QueryDelivery protocol message
- Add `QueryDelivery { blob_hash }` to `EprRequest` enum in `epr_protocol.rs`
- Add `DeliveryInfo { serves_extracted, serves_compressed, cache_tier, warm }` to `EprResponse`
- Handle in EPR behaviour: query local ExtractionCache state, return info
- Backward compatible: old peers return `Error("unknown variant")`

### Task 2: EPR knownLocations capability vocabulary
- When broadcasting EPR Document updates, include delivery capabilities in `capabilities` array
- Vocabulary: `"serves_extracted"`, `"serves_compressed"`, `"warm:{blob_hash}"`
- Capabilities update when extraction cache adds/evicts content

### Task 3: Multi-peer scoring in ContentResolverService
- Extend `content-resolver.ts` to score peers from knownLocations
- Scoring factors: network proximity (LAN > WAN > relay), delivery capability, recency, tier
- Return sorted list of peer candidates

### Task 4: SW peer fallback chain
- Extend SW fetch handler to try peers in scored order
- On each peer: `HEAD /_capability` to confirm, then fetch
- On failure: promote next peer
- Safety net: any peer with the blob → ZIP fetch + SW extract

### Task 5: Tauri LAN mesh (mDNS)
- Verify mDNS discovery surfaces delivery capabilities
- Tauri node A requests app file from Tauri node B on same LAN
- Direct HTTP to B's storage port — no doorway, no WAN

### Task 6: Browser discovery of local Tauri nodes
- Browser's SW discovers local Tauri node via EPR knownLocations (populated by doorway)
- Fetch from local Tauri node's HTTP port if available
- Falls back to doorway if local node unavailable

### Task 7: Stubs for future sprints
- Browser-to-browser WebRTC data channel: SW transport adapter interface
- Cross-WAN peer discovery without doorway: relay infrastructure
- Shefa bandwidth metering: economic coupling tracking

---

## Acceptance Criteria (from design doc)

1. Tauri node on LAN serves app files to another Tauri node via mDNS
2. Browser SW resolves multiple peers via EPR knownLocations
3. Peer scoring prefers LAN > doorway > remote
4. Fallback chain degrades gracefully (extracted -> compressed -> raw)
5. `QueryDelivery` libp2p message works, old peers degrade gracefully
