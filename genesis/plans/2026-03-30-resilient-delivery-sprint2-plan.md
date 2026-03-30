# Resilient HTML5 App Delivery — Sprint 2: Service Worker + Capability Negotiation

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.
>
> **REVISIT BEFORE EXECUTING:** This plan was written during Sprint 1 planning. Review and update based on Sprint 1 outcomes before executing.

**Goal:** Make every browser/Tauri instance a capable peer with client-side extraction. Peers advertise delivery capabilities so clients negotiate the best delivery mode.

**Architecture:** Two parts — (A) Extend NodeCapabilities with DeliveryCapabilities, broadcast via existing gossipsub. (B) Register a Service Worker that intercepts /apps/ requests, caches files locally, and falls back to ZIP extraction when peer only serves compressed.

**Tech Stack:** Rust (elohim-storage, steward/node), TypeScript (Angular SW), JSZip (client-side extraction)

**Design:** `genesis/plans/2026-03-30-resilient-html5-app-delivery-design.md`

**A2O Scenarios:**
- `genesis/a2o/features/delivery/client-resilience.feature` — 11 scenarios (SW registration, offline, capability negotiation, delivery modes)
- `genesis/a2o/features/federation/peer-advertisement.feature` — 16 scenarios (gossipsub heartbeat, neighbor table, dynamic state)
- `genesis/a2o/features/delivery/delivery-diagnostics.feature` — scenarios 7-11 (SW source reporting, layer disable, capability introspection)

---

## Part A: Capability Advertisement

### Task 1: DeliveryCapabilities struct (elohim-storage)
- Extend `identity.rs` NodeCapabilities with `DeliveryCapabilities`
- Add `CacheTier` enum (Projection, Extraction, BlobOnly)
- Add `ready_content: Vec<String>` (blob hashes with warm caches)
- Add `ready_content_hashes()` method to ExtractionCache

### Task 2: Gossipsub broadcast (steward/node)
- Extend `CapacityAnnouncement` in `capacity.rs` to include `DeliveryCapabilities`
- Serialize with MessagePack (existing framing)
- Broadcast on extraction cache add/evict events

### Task 3: Capability HEAD endpoint
- Add `HEAD /apps/{app_id}/_capability` to both doorway and storage
- Response headers: `X-Delivery-Mode`, `X-Blob-Hash`, `X-Cache-Tier`
- Doorway reports `Projection` when MongoDB cache is warm for this app
- Storage reports `Extraction` or `BlobOnly` based on ExtractionCache state

## Part B: Service Worker

### Task 4: SW registration
- Create `app/elohim-app/src/sw.ts` — the Service Worker
- Register in `main.ts` for both browser and Tauri WebView
- Scope: intercept `/apps/` fetch events

### Task 5: SW cache-first fetch
- On `/apps/` fetch: check CacheStorage first
- Key format: `{app_id}:{blob_hash}:{file_path}`
- On hit: return cached Response
- On miss: continue to network

### Task 6: SW capability probe
- Before bulk asset fetches, send `HEAD /apps/{app_id}/_capability`
- Parse response headers to determine delivery mode
- Choose strategy: individual file fetch vs. ZIP fetch + local extract

### Task 7: SW ZIP extraction (JSZip)
- When peer is `BlobOnly` or `compressed`: fetch ZIP blob
- Extract all files using JSZip
- Store each file in CacheStorage
- Return requested file

### Task 8: SW invalidation
- Listen for content update messages (via BroadcastChannel from Angular app)
- On new blob_hash for an app: evict all cached files for that app_id

### Task 9: Tauri verification
- Verify SW registers and works in Tauri WebView
- Verify offline capability (cached apps load without network)
- Verify fallback: storage direct → ZIP extract

---

## Acceptance Criteria (from design doc)

1. Service Worker registers in both browser and Tauri WebView
2. SW serves cached app files offline (zero network after first load)
3. SW capability probe chooses correct delivery mode per peer
4. When peer is blob-only, SW fetches ZIP and extracts locally
5. `DeliveryCapabilities` broadcast via existing gossipsub
6. `ready_content` updates dynamically as cache state changes
