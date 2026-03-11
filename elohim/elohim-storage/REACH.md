# elohim-storage: Reach Enforcement

elohim-storage is the P2P data plane. Its role in reach enforcement is to gate:

1. **Storage** - What gets stored locally and how
2. **Encryption** - Private content encrypted at rest
3. **Replication** - Who receives shards
4. **Delivery** - Who can request blobs

For the system-wide reach concept, see [../REACH.md](../REACH.md).

## The Core Mapping: Reach → Trust → Action

```
┌─────────────────────────────────────────────────────────────────────────┐
│                                                                         │
│   REACH LEVEL        TRUST REQUIRED       STORAGE BEHAVIOR              │
│   ───────────        ──────────────       ────────────────              │
│                                                                         │
│   private      →     Self only      →     Encrypted, my devices only   │
│   invited      →     Explicit list  →     Encrypted, named agents      │
│   local        →     Family         →     Encrypted, family cluster    │
│   neighborhood →     Extended       →     Cleartext, extended network  │
│   municipal    →     Community      →     Cleartext, community nodes   │
│   bioregional  →     Community      →     Cleartext, community nodes   │
│   regional     →     Community      →     Cleartext, community nodes   │
│   commons      →     Anyone         →     Cleartext, any willing node  │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

## Storage: What Gets Stored

### Encryption at Rest

| Reach | Encrypted | Key Holder |
|-------|-----------|------------|
| private | Yes | Beneficiary only |
| invited | Yes | Beneficiary + invited agents |
| local | Yes | Family cluster key |
| neighborhood+ | No | N/A (cleartext) |
| commons | No | N/A (cleartext) |

**Implementation:**

```rust
fn store_blob(&self, data: &[u8], metadata: &BlobMetadata) -> Result<String> {
    let stored_data = match metadata.reach.as_str() {
        "private" => {
            // Encrypt with beneficiary's public key
            encrypt_for_agent(data, &metadata.beneficiary)?
        }
        "invited" => {
            // Encrypt with shared key, distribute key to invited list
            let shared_key = generate_shared_key();
            self.distribute_key(&shared_key, &metadata.invited_agents)?;
            encrypt_symmetric(data, &shared_key)?
        }
        "local" => {
            // Encrypt with family cluster key
            encrypt_symmetric(data, &self.cluster_key)?
        }
        _ => {
            // Commons and above: cleartext
            data.to_vec()
        }
    };

    self.blob_store.put(&stored_data)
}
```

### LRU Eviction Priority

When storage is full, evict by reach level (protect private, shed commons):

```
Eviction order (first to evict → last):
  commons → regional → bioregional → municipal → neighborhood → local → invited → private
```

**Implementation:**

```rust
fn eviction_priority(reach: &str) -> u8 {
    match reach {
        "commons" => 0,      // Evict first (can fetch from network)
        "regional" => 1,
        "bioregional" => 2,
        "municipal" => 3,
        "neighborhood" => 4,
        "local" => 5,
        "invited" => 6,
        "private" => 7,      // Evict last (may be only copy)
        _ => 0,
    }
}

fn get_eviction_candidates(&self, needed_bytes: u64) -> Vec<BlobMetadata> {
    let mut candidates = self.metadata.list_all()?;

    // Sort by: eviction_priority ASC, then last_accessed ASC
    candidates.sort_by(|a, b| {
        let priority_cmp = eviction_priority(&a.reach).cmp(&eviction_priority(&b.reach));
        if priority_cmp == std::cmp::Ordering::Equal {
            a.last_accessed.cmp(&b.last_accessed)
        } else {
            priority_cmp
        }
    });

    candidates
}
```

## Replication: Who Gets Shards

### Reach → Replication Target

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    REPLICATION TOPOLOGY BY REACH                        │
│                                                                         │
│   private:       Only replicate to beneficiary's other devices          │
│                  ┌─────┐                                                │
│                  │ Me  │──► My laptop, My phone, My backup drive        │
│                  └─────┘                                                │
│                                                                         │
│   invited:       Replicate to explicitly invited agents                 │
│                  ┌─────┐                                                │
│                  │ Me  │──► Alice, Bob (named in invite list)           │
│                  └─────┘                                                │
│                                                                         │
│   local:         Replicate within family cluster                        │
│                  ┌─────────────────────────────┐                        │
│                  │ Family Cluster              │                        │
│                  │  Mom ◄──► Dad ◄──► Kid     │                        │
│                  └─────────────────────────────┘                        │
│                                                                         │
│   neighborhood:  Replicate to extended trust network                    │
│                  Family + Extended friends + Geographic neighbors       │
│                                                                         │
│   municipal+:    Replicate to community and beyond                      │
│                  Anyone with Community trust level or higher            │
│                                                                         │
│   commons:       Replicate to anyone willing to store                   │
│                  The whole network can participate                      │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### Trust Level Mapping

```rust
fn reach_to_minimum_trust(reach: &str) -> TrustLevel {
    match reach {
        "private" => TrustLevel::Self_,  // Special: same agent only
        "invited" => TrustLevel::Invited, // Special: explicit list
        "local" => TrustLevel::Family,
        "neighborhood" => TrustLevel::Extended,
        "municipal" | "bioregional" | "regional" => TrustLevel::Community,
        "commons" => TrustLevel::Network,
        _ => TrustLevel::Network,
    }
}

fn should_replicate_to(&self, blob: &BlobMetadata, peer: &ClusterMember) -> bool {
    match blob.reach.as_str() {
        "private" => {
            // Only replicate to same agent's other devices
            peer.agent_pubkey == blob.beneficiary
        }
        "invited" => {
            // Only replicate to explicitly invited agents
            blob.invited_agents.contains(&peer.agent_pubkey)
        }
        _ => {
            // For other levels, check trust threshold
            let required = reach_to_minimum_trust(&blob.reach);
            peer.trust_level >= required
        }
    }
}
```

### Reed-Solomon Shard Distribution

For RS-encoded blobs, distribute shards based on reach:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    RS SHARD DISTRIBUTION BY REACH                       │
│                                                                         │
│   commons (rs-4-7):                                                     │
│   ┌───────┐  Shard 1 → Any peer (Network trust)                        │
│   │ Blob  │  Shard 2 → Any peer                                        │
│   │  RS   │  Shard 3 → Any peer                                        │
│   │ 4+3   │  Shard 4 → Any peer                                        │
│   └───────┘  Shard 5 → Any peer (parity)                               │
│              Shard 6 → Any peer (parity)                               │
│              Shard 7 → Any peer (parity)                               │
│                                                                         │
│   local (rs-4-7):                                                       │
│   ┌───────┐  Shard 1 → Family member A                                 │
│   │ Blob  │  Shard 2 → Family member B                                 │
│   │  RS   │  Shard 3 → Family member C                                 │
│   │ 4+3   │  Shard 4 → Self (keep one)                                 │
│   └───────┘  Shard 5 → Family member A (parity)                        │
│              Shard 6 → Family member B (parity)                        │
│              Shard 7 → Family member C (parity)                        │
│                                                                         │
│   private (rs-4-7):                                                     │
│   ┌───────┐  All shards → Only my devices                              │
│   │ Blob  │  (Encrypted, distributed across my laptop, phone, NAS)     │
│   │  RS   │                                                             │
│   │ 4+3   │  If I only have 1 device: keep all shards locally          │
│   └───────┘  (Resilience limited by device count)                      │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

## Delivery: Who Can Request Blobs

### Request Validation

Before serving a blob, validate requester against reach:

```rust
fn can_serve_blob(&self, hash: &str, requester: &RequesterContext) -> Result<bool> {
    let metadata = self.metadata.get(hash)?
        .ok_or(StorageError::NotFound)?;

    match metadata.reach.as_str() {
        "private" => {
            // Only the beneficiary
            Ok(requester.agent_id == metadata.beneficiary)
        }
        "invited" => {
            // Beneficiary or explicitly invited
            Ok(requester.agent_id == metadata.beneficiary
                || metadata.invited_agents.contains(&requester.agent_id))
        }
        "local" => {
            // Family cluster members
            Ok(self.cluster.is_family_member(&requester.agent_id))
        }
        "neighborhood" => {
            // Extended trust network
            Ok(self.cluster.trust_level(&requester.agent_id) >= TrustLevel::Extended)
        }
        "municipal" | "bioregional" | "regional" => {
            // Community trust or higher
            Ok(self.cluster.trust_level(&requester.agent_id) >= TrustLevel::Community)
        }
        "commons" => {
            // Anyone
            Ok(true)
        }
        _ => Ok(false),
    }
}
```

### HTTP API Gating

```rust
// In http.rs route handler
async fn get_blob(
    State(state): State<AppState>,
    Path(hash): Path<String>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    // Extract requester from auth header
    let requester = extract_requester(&headers)?;

    // Check reach permission
    if !state.storage.can_serve_blob(&hash, &requester)? {
        return Err(StatusCode::FORBIDDEN);
    }

    // Decrypt if necessary
    let blob = state.storage.get_blob(&hash)?;
    let decrypted = state.storage.decrypt_for_requester(&blob, &requester)?;

    Ok(Response::new(decrypted))
}
```

## Sovereignty Mode Integration

Sovereignty mode provides the outer boundary; reach provides inner gating:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    SOVEREIGNTY × REACH                                  │
│                                                                         │
│   Sovereignty Mode sets WHO THIS NODE SERVES:                           │
│                                                                         │
│   Laptop      → Serves no one (even if content is commons)             │
│   HomeNode    → Serves family only                                      │
│   HomeCluster → Serves cluster members                                  │
│   Network     → Serves anyone                                           │
│                                                                         │
│   Reach sets WHO CAN ACCESS SPECIFIC CONTENT:                           │
│                                                                         │
│   Even in Network mode, private content only goes to beneficiary        │
│   Even in HomeNode mode, commons content can be served to family        │
│                                                                         │
│   BOTH must pass:                                                       │
│                                                                         │
│   can_serve(blob, requester) =                                          │
│       sovereignty.should_serve(requester)   // Node-level gate          │
│       && reach_allows(blob.reach, requester) // Content-level gate      │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

```rust
fn can_serve(&self, blob: &BlobMetadata, requester: &str) -> bool {
    // Gate 1: Does sovereignty mode allow serving this requester at all?
    if !self.sovereignty.should_serve(Some(requester)) {
        return false;
    }

    // Gate 2: Does content reach allow this requester?
    self.reach_allows(&blob.reach, requester)
}
```

## Implementation Status

| Component | Status | Notes |
|-----------|--------|-------|
| Reach field in metadata | ✅ Done | `BlobMetadata.reach` |
| Reach field in manifest | ✅ Done | `ShardManifest.reach` |
| Encryption at rest | ❌ Not started | Need key management |
| LRU by reach priority | ❌ Not started | Currently time-based only |
| Replication gating | 🔄 Partial | Sovereignty checks exist, reach mapping needed |
| Delivery gating | 🔄 Partial | `should_serve()` exists, needs reach integration |
| RS shard distribution by reach | ❌ Not started | Shards stored locally only |
| Trust level mapping | ✅ Done | `TrustLevel` enum in cluster.rs |

## Migration Path

### Phase 1: Delivery Gating
- Add `can_serve_blob()` with reach checks to HTTP handlers
- Integrate with existing `should_serve()` sovereignty check

### Phase 2: Replication Gating
- Add `should_replicate_to()` with reach→trust mapping
- Filter peer list before shard distribution

### Phase 3: Encryption
- Implement key management for private/invited/local
- Encrypt before storage, decrypt on retrieval
- Key distribution via Holochain DNA

### Phase 4: LRU by Reach
- Modify eviction to consider reach priority
- Protect private content from eviction

### Phase 5: Distributed RS
- Implement P2P shard transfer
- Gate shard recipients by reach→trust mapping

## Related Documentation

- [../REACH.md](../REACH.md) - System-wide reach concept
- [P2P-ARCHITECTURE.md](./P2P-ARCHITECTURE.md) - Dual-plane architecture
- [EDGE-ARCHITECTURE.md](./EDGE-ARCHITECTURE.md) - Performance layer
- [../doorway/REACH.md](../doorway/REACH.md) - Doorway reach enforcement
