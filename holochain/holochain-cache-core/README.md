# Holochain Cache Core

High-performance Rust/WASM cache module for Elohim Protocol content.

## Status

**Rust WASM module**: ✅ Complete (available for future use)
**TypeScript integration**: ✅ Complete via O(1) LRU in `BlobCacheTiersService`

The TypeScript implementation now uses O(1) LRU eviction (Map insertion-order).
The WASM module is available if additional performance is needed.

## Build

```bash
cargo build --target wasm32-unknown-unknown --release
cargo test
```

## Features

- **O(log n)** LRU eviction via BTreeMap time index
- **Reach-level isolation**: 8 independent caches (private → commons)
- **Elohim Protocol metadata**: domain, epic, custodian, mastery, priority

## API

```rust
// Core caches
BlobCache::new(max_size_bytes)        // O(1) LRU cache
ChunkCache::new(max_size, ttl_ms)     // TTL-based cache
ReachAwareCache::new(size_per_reach)  // 8 isolated LRU caches

// Operations
cache.put(hash, size, reach, domain, epic, priority)
cache.get(hash) / cache.has(hash) / cache.delete(hash)
cache.stats() / cache.clear()

// Utilities
calculate_priority(reach, proximity, bandwidth, steward, affinity, age_penalty)
calculate_freshness(mastery_level, age_seconds)
```

## Reach Levels

```
0 = Private      4 = Municipal
1 = Invited      5 = Bioregional
2 = Local        6 = Regional
3 = Neighborhood 7 = Commons
```

## Mastery Decay Rates

```
0 NotStarted  → 0.0/day     4 Apply    → 0.015/day
1 Seen        → 0.05/day    5 Analyze  → 0.01/day
2 Remember    → 0.03/day    6 Evaluate → 0.008/day
3 Understand  → 0.02/day    7 Create   → 0.005/day
```
