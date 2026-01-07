# Sparse DHT Pattern Deployment Review

## Summary

This deployment introduces the **manifest pattern** for efficient DHT usage: content bodies are stored in elohim-storage (blob store), while only lightweight manifests (metadata + hash references) are stored in the Holochain DHT.

**Target:** https://alpha.elohim.host/lamad

**Commits:**
- `403ddd46` - feat: Add sparse DHT pattern with P2P blob sync and elohim-node scaffold
- `8307f937` - fix(ci): Make schema type generation optional in genesis pipeline

---

## What Changed

### 1. Genesis Seeder (genesis/seeder/)

**New manifest pattern flow:**
```
Seed Data (JSON) → Upload blobs to elohim-storage → Create DHT entries with hash refs
```

**Key changes:**
- `seed.ts` - Pre-flight validation, format detection, blob sync before DHT writes
- `genesis-pack.ts` - Pre-computes content-addressed blobs at build time
- `verify-seed.ts` - Post-seeding validation
- `doorway-client.ts` - HTTP client for blob upload + import queue

**Pre-packed blobs:** `genesis/blobs/` contains ~3,500 SHA256-addressed content files

### 2. Content Store Zome (holochain/dna/elohim/zomes/content_store/)

**Debug logging for batch routing:**
```rust
// lib.rs - process_import_chunk now logs batch_type before routing
debug!("process_import_chunk: batch_type='{}'", batch.batch_type);

// Added fallback pattern for robustness
match batch.batch_type.as_str() {
    "paths" | "path" => { /* path processing */ }
    _ => { /* content processing */ }
}
```

**New fields in Content struct:**
- `blob_cid: Option<String>` - CID for content body in elohim-storage
- `content_size_bytes: Option<u64>` - Size for bandwidth estimation
- `content_hash: Option<String>` - SHA256 of content body

### 3. Doorway Gateway (holochain/doorway/)

**Blob proxy with shard fallback:**
- `GET /store/{address}` - Serves blobs with CID/SHA256/hex address support
- Range request support for video seeking (HTTP 206)
- Shard resolution fallback for distributed content

**Address formats accepted:**
- CID: `bafkreihdwdcefgh...`
- SHA256 prefixed: `sha256-a7ffc6f8...`
- Raw hex: `a7ffc6f8...`

### 4. elohim-storage (holochain/elohim-storage/)

**P2P sync engine (dormant but implemented):**
- Automerge-based document sync
- libp2p Kademlia DHT for peer discovery
- Stream-based change propagation
- `--enable-p2p` flag (disabled by default)

**Blob store enhancements:**
- Metadata storage with verification
- Import API for bulk operations
- WebSocket progress reporting

### 5. Angular Client (elohim-app/)

**Already configured for blob storage:**
- `BlobManagerService.getBlobUrl(hash)` - Strategy-aware URL construction
- Doorway mode: `https://doorway-dev.elohim.host/store/{hash}`
- Direct mode: `http://localhost:8090/store/{hash}` (Tauri)

---

## Deployment Verification Checklist

### Pre-Deployment

- [ ] Genesis pipeline passes (build #356+)
- [ ] Holochain pipeline passed (DNA compiled)
- [ ] Edgenode deployed with updated doorway

### Pipeline Stages to Watch

1. **Install Seeder** - npm ci should complete
2. **Generate Schema Types** - Should skip gracefully (uses committed constants)
3. **Verify Target Health** - Checks conductor + storage connectivity
4. **Seed Database** - Content + paths import (~3,500 items)
5. **Verify Seeding** - Confirms data written to DHT

### Post-Deployment Verification

```bash
# 1. Check doorway health
curl https://doorway-dev.elohim.host/health

# 2. Check doorway status (conductor + storage)
curl https://doorway-dev.elohim.host/status | jq

# 3. Test blob retrieval (any content hash from seed data)
curl -I https://doorway-dev.elohim.host/store/sha256-$(cat genesis/blobs/* | head -1 | sha256sum | cut -d' ' -f1)

# 4. Test content loading in app
open https://alpha.elohim.host/lamad
```

### Expected Behavior

1. **Content Viewer** loads markdown content from doorway blob endpoint
2. **Path Navigator** displays paths with correct step counts
3. **Images** (thumbnails) load from `/images/` static assets
4. **No 404s** in browser console for content requests

---

## Rollback Plan

If issues arise:

1. **Seeding failure:** Re-run genesis pipeline with `SEED_DATA=true`
2. **Blob not found:** Check elohim-storage logs, verify blob was uploaded
3. **DHT entry missing:** Content may need re-seeding for specific IDs

```bash
# Re-seed specific content
SEED_IDS="manifesto,elohim-protocol" npm run seed
```

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         ALPHA.ELOHIM.HOST                                │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│   Browser                                                                │
│      │                                                                   │
│      ├── GET /lamad/path/elohim-protocol ──────────────────────────────┐│
│      │                                                                  ││
│      ▼                                                                  ││
│   Angular App                                                           ││
│      │                                                                  ││
│      ├── ContentResolver.getPath() ───► Doorway WS ───► Conductor      ││
│      │                                      │              │            ││
│      │                                      │         DHT Manifest      ││
│      │                                      │         (metadata only)   ││
│      │                                      │              │            ││
│      └── BlobManager.downloadBlob() ──► Doorway /store/{hash}          ││
│                                              │                          ││
│                                              ▼                          ││
│                                       elohim-storage                    ││
│                                       (blob bodies)                     ││
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Key Files Changed

| Component | File | Change |
|-----------|------|--------|
| Seeder | `genesis/seeder/src/seed.ts` | Manifest pattern, blob sync |
| Seeder | `genesis/seeder/src/genesis-pack.ts` | Blob pre-packing |
| Zome | `holochain/dna/elohim/zomes/content_store/src/lib.rs` | Debug logging, batch routing |
| Zome | `holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` | blob_cid field |
| Doorway | `holochain/doorway/src/routes/blob.rs` | Shard resolution |
| Storage | `holochain/elohim-storage/src/p2p/mod.rs` | P2P sync engine |
| CI | `genesis/Jenkinsfile` | Optional schema generation |

---

## Questions for Review

1. **Blob cache warmup:** Should doorway pre-fetch popular blobs on startup?
2. **P2P activation:** Ready to enable `--enable-p2p` in production?
3. **Migration:** Existing DHT entries with inline content - lazy migration on access?
