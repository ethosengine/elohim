# Holochain Cache Core - Quick Start Guide

## 5-Minute Setup

### Step 1: Build WASM Module (2 minutes)

```bash
cd holochain-cache-core
wasm-pack build --target bundler --release
```

### Step 2: Copy to Angular Project (1 minute)

```bash
cp -r holochain-cache-core/pkg/* elohim-app/src/lib/holochain-cache-core/
```

### Step 3: Register Service (1 minute)

```typescript
// app.config.ts
import { HolochainCacheWasmService } from './lamad/services/holochain-cache-wasm.service';

export const appConfig: ApplicationConfig = {
  providers: [HolochainCacheWasmService, ...otherProviders],
};
```

### Step 4: Start Using (1 minute)

```typescript
// In your component
constructor(private cache: BlobCacheTiersService) {}

// Cache a blob with reach-aware metadata
this.cache.setBlob(
  'content-hash',
  blobData,
  7,                      // reach: commons (0-7)
  'elohim-protocol',      // domain
  'governance',           // epic
  'custodian-agent-id',   // optional custodian
  2,                      // steward tier (1-4)
  3,                      // mastery level (0-7)
  10,                     // proximity score (-100 to +100)
  3,                      // bandwidth class (1-4)
  0.8,                    // affinity match (0.0-1.0)
);

// Retrieve from cache
const blob = this.cache.getBlob('content-hash', 7);
```

---

## Core Concepts

### Reach Levels (0-7)
```
0 = Private      (only beneficiary)
1 = Invited      (explicit invites)
2 = Local        (family/household)
3 = Neighborhood (street block)
4 = Municipal    (city/town)
5 = Bioregional  (watershed/ecosystem)
6 = Regional     (state/province)
7 = Commons      (global/public)
```

### Domains
- `elohim-protocol` - Core Elohim Protocol
- `fct` - Foundations for Christian Technology
- `ethosengine` - Ethos Engine
- `other` - Custom domains

### Epics
- `governance` - Governance systems
- `autonomous_entity` - Autonomous entities
- `public_observer` - Public observation
- `social_medium` - Social media
- `value_scanner` - Value scanning
- `economic_coordination` - Economic coordination
- `lamad` - LAMAD learning paths
- `other` - Custom epics

### Mastery Levels (0-7)
```
0 = NotStarted    (no decay)
1 = Seen          (0.05 daily decay)
2 = Remember      (0.03 daily decay)
3 = Understand    (0.02 daily decay)
4 = Apply         (0.015 daily decay)
5 = Analyze       (0.01 daily decay)
6 = Evaluate      (0.008 daily decay)
7 = Create        (0.005 daily decay)
```

Higher mastery = slower freshness decay = content stays relevant longer

### Steward Tiers (1-4)
```
1 = Caretaker  (basic stewardship)
2 = Curator    (active curation)
3 = Expert     (domain expertise)
4 = Pioneer    (original research)
```

---

## Common Operations

### Preload Learning Path Content

```typescript
async preloadPath(domain: string, epic: string, reachLevel: number) {
  // Get content list from API
  const items = await this.api.getPathContent(domain, epic);

  for (const item of items) {
    const blob = await this.download(item.hash);
    this.cache.setBlob(
      item.hash,
      blob,
      reachLevel,
      domain,
      epic,
      item.custodian,
      item.stewardTier,
      0, // Will be updated per-user
      item.proximityScore,
      item.bandwidthClass,
      item.affinityMatch,
    );
  }
}
```

### Get All Content for Domain/Epic

```typescript
// Returns hashes of all cached items for this domain/epic
const hashes = this.cache.queryByDomainEpic('elohim-protocol', 'governance');
```

### Check Custodian Replicas

```typescript
// See what this custodian has replicated
const replicas = this.cache.getBlobsByCustodian('agent-id');
console.log(`${replicas.length} items replicated by this custodian`);
```

### Monitor Cache Health

```typescript
const health = this.cache.getHealthReport();
console.log(`Cache: ${health.totalItems} items, ${health.hitRate.toFixed(1)}% hit rate`);

// See distribution across reach levels
const dist = this.cache.getReachDistribution();
console.log('Cached items by reach level:', dist);
```

---

## Performance Tips

### 1. Batch Inserts
```typescript
// Instead of:
for (const item of items) {
  cache.setBlob(...); // Calls WASM each time
}

// Do this:
items.forEach(item => {
  // Batching happens automatically in WASM
  cache.setBlob(...);
});
```

### 2. Use Domain/Epic Queries
```typescript
// Fast O(log n) lookup in hash index
const hashes = cache.queryByDomainEpic('fct', 'governance');

// Instead of iterating through entire cache
const allBlobs = cache.getBlobsByDomainEpic('fct', 'governance');
```

### 3. Preload During App Startup
```typescript
// In app initialization
constructor(private cache: BlobCacheTiersService) {
  this.cache.ensureReady().then(() => {
    // Preload commons content after WASM loads
    this.preloadCommons();
  });
}
```

### 4. Monitor High-Traffic Reach Levels
```typescript
// Commons content gets more traffic
// Allocate more cache per reach level accordingly
const dist = cache.getReachDistribution();
if (dist[7].items > 1000) {
  console.warn('Commons cache is full, consider optimization');
}
```

---

## Troubleshooting

### "WASM module not ready"
```typescript
// Always wait for initialization
await this.cache.ensureReady();
if (!this.cache.isReady()) {
  console.error('WASM failed to load');
}
```

### Cache hits are low
```typescript
// Check if you're using correct reach level
const entry = cache.get('hash', 7); // Must match stored reach

// Verify domain/epic match exactly
cache.queryByDomainEpic('elohim-protocol', 'governance'); // Case-sensitive
```

### Memory usage growing
```typescript
// Check stats per reach level
for (let reach = 0; reach <= 7; reach++) {
  const stats = cache.getReachStats(reach);
  if (stats.totalSizeBytes > 500 * 1024 * 1024) {
    console.warn(`Reach ${reach} is ${stats.totalSizeBytes / 1024 / 1024}MB`);
  }
}

// Clear old content
cache.clear(); // Start fresh
```

---

## Architecture Overview

```
┌─────────────────────────────────────┐
│   Angular Component                 │
│   (LearningPathComponent, etc.)      │
└────────────┬────────────────────────┘
             │
┌────────────▼────────────────────────┐
│  BlobCacheTiersService              │
│  - Tier 1: Metadata (JS)            │
│  - Tier 2: Blobs (WASM)             │
│  - Tier 3: Chunks (WASM)            │
└────────────┬────────────────────────┘
             │
┌────────────▼────────────────────────┐
│  HolochainCacheWasmService          │
│  - Wrapper for WASM module          │
│  - Type conversion                  │
│  - Index management                 │
└────────────┬────────────────────────┘
             │
┌────────────▼────────────────────────┐
│  holochain-cache-core (WASM)        │
│  - ReachAwareBlobCache              │
│  - 8x BlobCache (one per reach)     │
│  - Domain/Epic index                │
│  - Custodian index                  │
│  - O(log n) operations              │
└─────────────────────────────────────┘
```

---

## Next: Advanced Usage

For advanced topics like:
- Parallel integrity verification
- Custodian selection algorithms
- Steward Economy integration
- Custom serialization

See: `HOLOCHAIN_CACHE_CORE_IMPLEMENTATION.md`

---

## Build & Deploy

### Development
```bash
# Watch for changes
wasm-pack build --dev --target bundler

# Smaller binary for production
wasm-pack build --release --target bundler
```

### Production Checklist
- ✅ WASM served with `application/wasm` MIME type
- ✅ Gzip compression enabled
- ✅ Browser cache headers set
- ✅ Error handling in place
- ✅ Fallback to JS if WASM fails
- ✅ Monitoring/alerting configured

---

## Support

For issues or questions:
1. Check console logs: `[HolochainCacheWasm]` prefix
2. Review test cases in `holochain-cache-core/src/lib.rs`
3. Check health report: `cache.getHealthReport()`
4. Monitor reach distribution: `cache.getReachDistribution()`
