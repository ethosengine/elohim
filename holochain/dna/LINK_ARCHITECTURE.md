# Link Architecture: DHT Links vs Projection Queries

## The 256 LinkType Limit

Holochain's `#[hdk_link_types]` uses a `u8` discriminant, limiting each zome to **256 link types**.
The original monolithic `content_store_integrity` reached **255 link types** - the absolute limit.

This document establishes the architectural split between:
1. **DHT Links** - Essential for integrity and real-time signals
2. **Projection Queries** - UX enrichment via MongoDB projections

---

## Philosophy: When to Use Links vs Queries

### Use DHT Links For:

| Use Case | Example | Why Link? |
|----------|---------|-----------|
| **Identity binding** | `AgentKeyToHuman` | One-to-one auth binding must be DHT-verified |
| **Structural navigation** | `PathToStep`, `ChapterToStep` | Entry graph traversal |
| **Ownership proof** | `OperatorToDoorway` | Validation needs to verify author owns doorway |
| **Signal emission** | `IdToContent` | post_commit needs link to find entry for signal |
| **Cross-DNA references** | `ProgressToPath` | Bridge calls need deterministic lookup |
| **Time-sensitive state** | `DoorwayToHeartbeat` | Recent heartbeats need DHT presence |

### Use Projection Queries For:

| Use Case | Example | Why Query? |
|----------|---------|------------|
| **Filtering by attribute** | "Get all content with type='video'" | MongoDB: `{contentType: 'video'}` |
| **Sorting** | "Get paths by difficulty, newest first" | MongoDB: `.sort({difficulty: 1, createdAt: -1})` |
| **Pagination** | "Get page 3 of 50 results" | MongoDB: `.skip(100).limit(50)` |
| **Aggregations** | "Count attestations by category" | MongoDB: `$group` pipeline |
| **Full-text search** | "Find content mentioning 'governance'" | MongoDB text index |
| **Complex joins** | "Get content with author's display name" | MongoDB `$lookup` |
| **Statistics** | "Average completion rate per path" | MongoDB aggregation |

### The Signal Rule

**If a link type exists ONLY to enable queries, it should be a projection query instead.**

Links that follow this pattern are candidates for removal:
```
{Entity}By{Attribute}  →  Projection query
{Entity}ByStatus       →  Projection query
{Entity}ByType         →  Projection query
{Entity}ByCategory     →  Projection query
```

---

## Multi-DNA Architecture

### DNA Responsibilities

| DNA | Domain | Entry Types | Link Purpose |
|-----|--------|-------------|--------------|
| **lamad** | Content & Learning | Content, Path, Step, Chapter | Content graph navigation |
| **infrastructure** | Network Ops | Doorway, Heartbeat, Summary | Operator auth, health signals |
| **imagodei** | Identity | Human, Agent, Relationship, Attestation | Identity binding, social graph |

### Link Ownership by DNA

#### Infrastructure DNA Links (8 types)
```rust
IdToDoorway           // Essential: lookup by ID
OperatorToDoorway     // Essential: auth verification
RegionToDoorway       // Query candidate, but useful for bootstrap routing
ReachToDoorway        // Query candidate
TierToDoorway         // Query candidate
DoorwayToHeartbeat    // Essential: time-series navigation
DoorwayToSummary      // Essential: permanent history
SummaryByDate         // Essential: cross-doorway date queries
```

#### Imago Dei DNA Links (21 types)
```rust
// Identity binding (essential)
IdToHuman, IdToAgent
AgentKeyToHuman, AgentKeyToAgent
HumanByExternalId

// Progress navigation (essential)
HumanToProgress, AgentToProgress
ProgressToPath

// Social graph (essential - custody/recovery)
IdToHumanRelationship
AgentToRelationship
RelationshipPendingConsent
RelationshipWithCustody

// Query candidates (consider projection)
HumanByAffinity, AgentByAffinity
HumanRelationshipByIntimacy
HumanRelationshipByType
AttestationByCategory, AttestationByType
```

#### Lamad DNA Links (remaining ~220 types)
After removing infrastructure and imagodei links, lamad should focus on:
- Content graph navigation
- Path/Step/Chapter structure
- Blob media relationships
- REA economic events (if keeping Shefa in lamad)

---

## Migration Strategy

### Phase 1: Multi-DNA Split (Current)
- ✅ Create infrastructure DNA with Doorway types
- ✅ Create imagodei DNA with identity types
- ⏳ Remove duplicated types from lamad content_store_integrity

### Phase 2: Query Link Deprecation
1. Identify all `*By{Attribute}` links in lamad
2. Verify projection handles the query
3. Stop creating new links of deprecated types
4. Keep link type in enum (for DHT compatibility)
5. Mark as `// DEPRECATED: Use projection query`

### Phase 3: Signal-Only Links
For links that exist only for signals:
```rust
// Before: Link exists for query AND signal
IdToContent  // Used by get_content() AND post_commit signal

// After: Keep for signal, query via projection
IdToContent  // Used by post_commit signal only
             // Query: db.content.findOne({id: ...})
```

---

## Link Types to Remove from Lamad

These are now in infrastructure or imagodei DNAs:

### Doorway Links (move to infrastructure)
```rust
// REMOVE from content_store_integrity:
IdToDoorway
OperatorToDoorway
DoorwayToHeartbeat
DoorwayToSummary
```

### Identity Links (move to imagodei)
```rust
// REMOVE from content_store_integrity:
IdToHuman, HumanByAffinity, HumanToProgress, ProgressToPath
AgentKeyToHuman, HumanByExternalId
IdToAgent, AgentByType, AgentByAffinity, AgentToProgress
AgentProgressToPath, AgentKeyToAgent, ElohimByScope
AgentToAttestation, AttestationByCategory, AttestationByType
IdToHumanRelationship, AgentToRelationship
HumanRelationshipByIntimacy, HumanRelationshipByType
RelationshipPendingConsent, RelationshipWithCustody
HumanToMastery  // ContentMastery moved to imagodei
```

### Query-Only Links (deprecate, use projection)
```rust
// DEPRECATE in content_store_integrity (mark, don't remove):
TypeToContent           // Query: db.content.find({contentType: ...})
TagToContent            // Query: db.content.find({tags: ...})
AuthorToContent         // Query: db.content.find({author: ...})
PathByCreator           // Already removed
PathByDifficulty        // Already removed
PathByType              // Already removed
PathByTag               // Already removed
EventByAction           // Query: db.events.find({action: ...})
EventByLamadType        // Query: db.events.find({lamadEventType: ...})
ResourceBySpec          // Query: db.resources.find({conformsTo: ...})
// ... many more *By* patterns
```

---

## Implementation Checklist

- [ ] Remove 4 Doorway link types from lamad (in infrastructure now)
- [ ] Remove ~25 Identity link types from lamad (in imagodei now)
- [ ] Mark ~50 query-only links as DEPRECATED
- [ ] Update coordinator functions to use cross-DNA calls where needed
- [ ] Verify projection subscriber handles all query use cases
- [ ] Document cross-DNA bridge call patterns

---

## Appendix: Link Count by Domain

After proper split:

| DNA | Link Types | Purpose |
|-----|------------|---------|
| infrastructure | ~8 | Doorway federation |
| imagodei | ~21 | Identity & relationships |
| lamad | ~175 | Content, paths, REA (after cleanup) |

Total: ~204 (down from 255, with room to grow in each DNA)
