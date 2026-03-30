# Resilient HTML5 App Delivery — Sprint 3: P2P Mesh Delivery

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Peers serve peers. Client resolves multiple delivery peers via EPR, scores by capability and proximity, falls back gracefully through the ranked list. Doorway becomes one source among many.

**Architecture:** Extend EPR protocol with QueryDelivery message for peer capability probing over libp2p. Extend ContentResolverService with multi-peer scoring. Extend SW with peer fallback chain — try best peer, degrade through list, ZIP extraction as safety net.

**Tech Stack:** Rust (elohim-storage P2P epr_protocol, behaviour handler), TypeScript (ContentResolverService scoring, SW multi-peer fetch)

**Design:** `genesis/plans/2026-03-30-resilient-html5-app-delivery-design.md`

**A2O Scenarios:**
- `genesis/a2o/features/delivery/peer-mesh.feature` — 10 scenarios (LAN peer discovery, multi-peer resolution, fallback chain, QueryDelivery protocol)
- `genesis/a2o/features/elohim/network-health-posture.feature` — 19 scenarios (aggregate posture, attestation-gated introspection, elohim agent reasoning)

**Sprint 1+2 Outcomes That Inform This Plan:**
- Doorway projection cache live (Sprint 1) — MongoDB cache absorbs web2 traffic
- SW live (Sprint 2) — cache-first fetch, capability probe via HEAD, ZIP extraction fallback
- DeliveryCapabilities + CacheTier in identity.rs — structs ready for P2P advertisement
- ready_content_hashes() on ExtractionCache — reports warm content for gossipsub
- Capability probe proxies through doorway to storage — one node, one report
- ContentResolverService has known_locations with recency but no capability scoring
- mDNS discovery exists in libp2p (behaviour.rs) but peers not exposed to frontend

---

### Task 1: QueryDelivery EPR protocol message

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/epr_protocol.rs`

Add two new variants to the existing enums:

```rust
// In EprRequest (after GetDocument):
/// Query a peer's delivery capability for a specific blob
QueryDelivery {
    blob_hash: String,
},

// In EprResponse (after Error):
/// Delivery capability info for a specific blob
DeliveryInfo {
    serves_extracted: bool,
    serves_compressed: bool,
    cache_tier: String,    // "projection" | "extraction" | "blob-only"
    warm: bool,            // this specific blob is extracted and ready
},
```

These are new variants on existing MessagePack-serialized enums. Backward compatible — old peers receiving unknown variants return `EprResponse::Error("unknown request variant")`.

**Tests:**
- Roundtrip encode/decode QueryDelivery
- Roundtrip encode/decode DeliveryInfo
- Verify old response variant still decodes correctly (backward compat)

**Verify:** `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test epr`

**Commit:** `feat(storage): add QueryDelivery/DeliveryInfo EPR protocol variants`

---

### Task 2: Handle QueryDelivery in EPR request handler

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` (~line 1981 in `handle_epr_request`)

Add a match arm for QueryDelivery that:
1. Looks up the blob_hash in ExtractionCache
2. Checks `is_current()` for warm status
3. Returns DeliveryInfo with the node's actual capability

```rust
EprRequest::QueryDelivery { blob_hash } => {
    let warm = match &self.extraction_cache {
        Some(cache) => {
            // Check if any app uses this blob_hash and it's warm
            let hashes = cache.ready_content_hashes().await;
            hashes.contains(&blob_hash)
        }
        None => false,
    };

    let (serves_extracted, cache_tier) = match &self.extraction_cache {
        Some(_) => (warm, "extraction".to_string()),
        None => (false, "blob-only".to_string()),
    };

    EprResponse::DeliveryInfo {
        serves_extracted,
        serves_compressed: true, // all nodes can serve raw blobs
        cache_tier,
        warm,
    }
}
```

No reach authorization needed for capability queries — you're asking "can you serve this?" not "give me this content." The actual content fetch still goes through reach gates.

**Tests:**
- Handler returns warm=true when extraction cache has the hash
- Handler returns warm=false when extraction cache is empty
- Handler returns blob-only when no extraction cache

**Verify:** `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test`

**Commit:** `feat(storage): handle QueryDelivery in EPR protocol handler`

---

### Task 3: Expose discovered peers to frontend via HTTP

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs`

The frontend needs to know about peers discovered via mDNS/Kademlia. Add an endpoint:

```
GET /api/v1/peers/delivery
```

Returns JSON array of peers with their delivery capabilities:

```json
[
  {
    "peerId": "12D3KooW...",
    "multiaddrs": ["/ip4/192.168.1.50/tcp/9876"],
    "network": "lan",
    "capabilities": ["serves_extracted", "cache_tier:extraction", "warm:sha256-abc"],
    "lastSeen": 1711814400000,
    "httpPort": 8090
  }
]
```

This data comes from:
1. The gossipsub neighbor table (CapacityAnnouncements received)
2. mDNS discovered peers (local network)
3. Kademlia peer routing (remote peers)

Each peer's delivery capability strings are already in the CapacityAnnouncement. Parse them to determine `serves_extracted`, `cache_tier`, and `warm:{hash}` entries.

**Important:** Also add this route to `build_manifest()` so doorway auto-discovers it.

**Tests:**
- Returns empty array when no peers known
- Returns peer with correct fields when gossipsub announcement received

**Verify:** `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check`

**Commit:** `feat(storage): expose delivery peers via HTTP for frontend discovery`

---

### Task 4: Multi-peer scoring in ContentResolverService

**Files:**
- Modify: `app/elohim-library/projects/elohim-service/src/cache/content-resolver.ts`

Extend the resolver to score peers based on delivery capability. Currently it sorts by (tier, priority, recency). Add capability-aware scoring:

```typescript
interface PeerScore {
  sourceId: string;
  baseUrl: string;
  score: number;      // higher = better
  network: 'lan' | 'wan' | 'relay';
  deliveryMode: 'extracted' | 'compressed' | 'blob-only';
  warm: boolean;
}

function scorePeer(peer: DeliveryPeer, contentHash: string): number {
  let score = 0;

  // Network proximity (biggest factor)
  if (peer.network === 'lan') score += 1000;
  else if (peer.network === 'wan') score += 500;
  else score += 100; // relay

  // Delivery capability
  if (peer.capabilities.includes('serves_extracted')) score += 200;
  if (peer.capabilities.includes('serves_compressed')) score += 50;

  // Warm cache for THIS content
  if (peer.capabilities.includes(`warm:${contentHash}`)) score += 300;

  // Recency (prefer recently-seen peers)
  const age = Date.now() - peer.lastSeen;
  if (age < 30000) score += 100;      // seen in last 30s
  else if (age < 90000) score += 50;  // seen in last 90s
  // else: stale, no bonus

  return score;
}
```

Add a method `scorePeersForContent(contentHash: string): PeerScore[]` that:
1. Fetches `/api/v1/peers/delivery` (or uses cached peer list)
2. Scores each peer
3. Returns sorted list (highest score first)

**Tests:**
- LAN + warm scores higher than WAN + warm
- WAN + extracted scores higher than WAN + compressed
- Stale peers get lower scores
- Empty peer list returns empty array

**Verify:** `cd app/elohim-library && pnpm test`

**Commit:** `feat(elohim-service): multi-peer scoring for delivery resolution`

---

### Task 5: SW multi-peer fallback chain

**Files:**
- Modify: `app/elohim-app/src/apps-sw.ts`

Extend `handleAppFetch` to try multiple peers in scored order. Currently it probes one peer (doorway) and either fetches extracted or ZIP. Change to:

```typescript
async function handleAppFetch(request: Request): Promise<Response> {
  const url = new URL(request.url);
  const pathParts = url.pathname.replace('/apps/', '').split('/');
  const appId = pathParts[0];
  const filePath = pathParts.slice(1).join('/');

  // 1. Local cache first (unchanged)
  const cache = await caches.open(CACHE_NAME);
  const cached = await cache.match(request);
  if (cached) return cached;

  // 2. Get ranked peer list
  const peers = await getDeliveryPeers(appId);

  // 3. Try peers in order
  for (const peer of peers) {
    try {
      if (peer.deliveryMode === 'extracted' && peer.warm) {
        // Peer has files ready — fetch individual file
        const resp = await fetch(`${peer.baseUrl}/apps/${appId}/${filePath}`);
        if (resp.ok) {
          cache.put(request, resp.clone());
          return resp;
        }
      } else if (peer.deliveryMode === 'compressed' || !peer.warm) {
        // Peer has ZIP only — fetch and extract
        const blobHash = peer.blobHash || (await probeCapability(appId)).blobHash;
        if (blobHash) {
          await fetchViaZipFromPeer(cache, appId, blobHash, peer.baseUrl);
          const extracted = await cache.match(request);
          if (extracted) return extracted;
        }
      }
    } catch {
      // Peer failed — try next
      continue;
    }
  }

  // 4. All peers exhausted — try default path (doorway)
  return fetchAndCache(cache, request);
}
```

Add `getDeliveryPeers(appId)` that:
1. Fetches `/api/v1/peers/delivery` (cached for 30s)
2. Filters for peers with the relevant content warm
3. Returns scored list including doorway as one entry

**Important:** Doorway is always in the list as a WAN peer with projection cache. It's not special — just one peer among many, scored by the same algorithm.

**Tests:**
- Manual: verify multi-peer fallback in network tab
- The a2o scenarios describe the expected behavior

**Commit:** `feat(app): SW multi-peer fallback chain for P2P app delivery`

---

### Task 6: Expose LAN peers to browser via doorway

**Files:**
- Modify: `doorway/doorway-service/src/server/http.rs` (route dispatch, if not auto-discovered via manifest)

The `/api/v1/peers/delivery` endpoint from Task 3 is on elohim-storage. For browser clients going through doorway, this route needs to be accessible. Two options:

1. **Auto-discovered via manifest** (preferred) — if Task 3 added it to `build_manifest()`, doorway's RouteRegistry already proxies it. Verify this works.

2. **Explicit route** (fallback) — if the manifest approach has issues, add a dedicated match arm.

Also: for LAN peers, the browser needs to know the peer's HTTP address (not just libp2p multiaddr). The peer list from Task 3 should include `httpPort` so the browser can construct `http://{ip}:{port}/apps/...`.

**Verify:** `curl http://localhost:8888/api/v1/peers/delivery` returns the peer list through doorway proxy.

**Commit:** `feat(doorway): verify peer delivery endpoint routes through manifest`

---

### Task 7: Backward compatibility test

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/epr_protocol.rs` (tests only)

Verify that:
1. An old peer (without QueryDelivery) that receives a QueryDelivery request returns `Error("unknown request variant")`
2. A new peer that receives old-format requests (Resolve, Announce) still works
3. The SW gracefully handles peers that don't support the capability endpoint (404 → skip peer, try next)

This is the `@regression` scenario from `peer-mesh.feature`: "Old peers handle QueryDelivery gracefully"

**Tests:**
- Serialize a QueryDelivery, attempt to deserialize as old enum → error
- Serialize old Resolve request, deserialize with new enum → works
- SW fetches from peer that returns 404 on /_capability → falls back to next peer

**Commit:** `test(storage): backward compatibility for QueryDelivery protocol`

---

### Task 8: Integration verification

**Step 1: Rust test suites**
```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test
cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib --bins
cd elohim/elohim-cache-core && cargo test
```

**Step 2: Clippy**
```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings
cd doorway/doorway-service && RUSTFLAGS="" cargo clippy -- -D warnings
```

**Step 3: TypeScript**
```bash
cd app/elohim-library/projects/elohim-service && pnpm test
cd app/elohim-app && pnpm run build:sw
```

**Step 4: Format**
```bash
cd doorway/doorway-service && cargo fmt --check
cd elohim/elohim-storage && cargo fmt --check
```

---

## Stubs for Future Sprints

These are documented but NOT implemented in Sprint 3:

- **Browser → Browser (WebRTC data channel):** The SW `getDeliveryPeers()` returns HTTP URLs. A future transport adapter adds WebRTC peers as an alternative transport, resolved through the same scoring algorithm.
- **Cross-WAN peer discovery without doorway:** Requires relay infrastructure maturity. Currently WAN peers are discovered through doorway's peer list.
- **Shefa bandwidth metering:** The EPR agreement authorizes caching. Metering the bandwidth contributed (for shefa economic events) is a future sprint.

## Key Files Reference

| File | Purpose |
|------|--------|
| `elohim/elohim-storage/src/p2p/epr_protocol.rs` | QueryDelivery/DeliveryInfo variants (Task 1) |
| `elohim/elohim-storage/src/p2p/mod.rs` | EPR request handler (Task 2) |
| `elohim/elohim-storage/src/http.rs` | /api/v1/peers/delivery endpoint (Task 3) |
| `app/elohim-library/.../cache/content-resolver.ts` | Multi-peer scoring (Task 4) |
| `app/elohim-app/src/apps-sw.ts` | Multi-peer fallback chain (Task 5) |
| `doorway/doorway-service/src/server/http.rs` | Verify peer route proxies (Task 6) |
